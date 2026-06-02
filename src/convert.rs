//! Convert a session transcript between Claude Code and OpenAI Codex on-disk
//! formats, so a session recorded by one CLI can be opened/continued in the
//! other. Best-effort: the conversation text (user + assistant turns) is
//! preserved; tool calls and images are flattened to text.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use chrono::Local;
use serde_json::{json, Value};

use crate::models::{SessionInfo, Source};
use crate::scanner::{claude_dir, codex_dir, codex_id_from_filename};

/// One normalized conversation turn.
struct Msg {
    role: String,
    text: String,
    ts: Option<String>,
}

/// Convert `session` to the *other* tool's format and return the written path.
pub fn convert_session(session: &SessionInfo) -> Result<PathBuf> {
    if session.cwd.trim().is_empty() {
        return Err(anyhow!("session has no working directory to convert into"));
    }
    match session.source {
        Source::Claude => {
            let msgs = read_claude_transcript(session)?;
            if msgs.is_empty() {
                return Err(anyhow!("no convertible messages in this session"));
            }
            write_codex_rollout(session, &msgs)
        }
        Source::Codex => {
            let msgs = read_codex_transcript(session)?;
            if msgs.is_empty() {
                return Err(anyhow!("no convertible messages in this session"));
            }
            write_claude_jsonl(session, &msgs)
        }
    }
}

/// `cwd` → Claude's project-dir name (`/a/b` → `-a-b`).
fn project_name_for(cwd: &str) -> String {
    let s = cwd.strip_prefix('/').unwrap_or(cwd);
    format!("-{}", s.replace('/', "-"))
}

/// Pull plain text out of a message `content` (a string, or an array of blocks).
fn extract_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => {
            let mut parts: Vec<String> = Vec::new();
            for b in arr {
                if let Some(t) = b.get("text").and_then(|x| x.as_str()) {
                    parts.push(t.to_string());
                } else if let Some(t) = b.get("content").and_then(|x| x.as_str()) {
                    parts.push(t.to_string());
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

fn read_claude_transcript(session: &SessionInfo) -> Result<Vec<Msg>> {
    let path = claude_dir()
        .join("projects")
        .join(project_name_for(&session.cwd))
        .join(format!("{}.jsonl", session.id));
    let content = fs::read_to_string(&path).map_err(|e| anyhow!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in content.lines() {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ty = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        if ty != "user" && ty != "assistant" {
            continue;
        }
        let Some(msg) = v.get("message") else {
            continue;
        };
        let role = msg
            .get("role")
            .and_then(|x| x.as_str())
            .unwrap_or(ty)
            .to_string();
        let text = extract_text(msg.get("content"));
        if text.trim().is_empty() {
            continue;
        }
        let ts = v
            .get("timestamp")
            .and_then(|x| x.as_str())
            .map(String::from);
        out.push(Msg { role, text, ts });
    }
    Ok(out)
}

fn find_codex_rollout(uuid: &str) -> Option<PathBuf> {
    let dir = codex_dir().join("sessions");
    for e in walkdir::WalkDir::new(&dir)
        .max_depth(5)
        .into_iter()
        .flatten()
    {
        let p = e.path();
        if let Some(f) = p.file_name().and_then(|s| s.to_str()) {
            // Exact UUID match (not a substring) so a shared prefix can't pick
            // the wrong rollout, and traversal order can't matter.
            if codex_id_from_filename(f).as_deref() == Some(uuid) {
                return Some(p.to_path_buf());
            }
        }
    }
    None
}

fn read_codex_transcript(session: &SessionInfo) -> Result<Vec<Msg>> {
    let path = find_codex_rollout(&session.id)
        .ok_or_else(|| anyhow!("codex rollout not found for {}", session.id))?;
    let content = fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("type").and_then(|x| x.as_str()) != Some("response_item") {
            continue;
        }
        let Some(p) = v.get("payload") else { continue };
        if p.get("type").and_then(|x| x.as_str()) != Some("message") {
            continue;
        }
        let role = p
            .get("role")
            .and_then(|x| x.as_str())
            .unwrap_or("user")
            .to_string();
        let text = extract_text(p.get("content"));
        if text.trim().is_empty() {
            continue;
        }
        let ts = v
            .get("timestamp")
            .and_then(|x| x.as_str())
            .map(String::from);
        out.push(Msg { role, text, ts });
    }
    Ok(out)
}

fn write_claude_jsonl(session: &SessionInfo, msgs: &[Msg]) -> Result<PathBuf> {
    let newid = gen_uuid(&session.id);
    let dir = claude_dir()
        .join("projects")
        .join(project_name_for(&session.cwd));
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{newid}.jsonl"));
    let mut buf = String::new();
    for m in msgs {
        let line = json!({
            "type": if m.role == "assistant" { "assistant" } else { "user" },
            "sessionId": newid,
            "cwd": session.cwd,
            "timestamp": m.ts.clone().unwrap_or_default(),
            "message": {
                "role": m.role,
                "content": [ { "type": "text", "text": m.text } ],
            },
        });
        buf.push_str(&line.to_string());
        buf.push('\n');
    }
    fs::write(&path, buf)?;
    Ok(path)
}

fn write_codex_rollout(session: &SessionInfo, msgs: &[Msg]) -> Result<PathBuf> {
    let newid = gen_uuid(&session.id);
    let now = Local::now();
    let dir = codex_dir()
        .join("sessions")
        .join(now.format("%Y").to_string())
        .join(now.format("%m").to_string())
        .join(now.format("%d").to_string());
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "rollout-{}-{}.jsonl",
        now.format("%Y-%m-%dT%H-%M-%S"),
        newid
    ));
    let ts_iso = now.to_rfc3339();
    let model = session.model.clone().unwrap_or_else(|| "gpt-5.5".into());

    let mut buf = String::new();
    buf.push_str(
        &json!({
            "timestamp": ts_iso,
            "type": "session_meta",
            "payload": {
                "id": newid,
                "timestamp": ts_iso,
                "cwd": session.cwd,
                "originator": "managecode_convert",
                "cli_version": "managecode",
                "source": "convert",
                "model_provider": "openai",
                "git": {}
            }
        })
        .to_string(),
    );
    buf.push('\n');
    buf.push_str(
        &json!({
            "timestamp": ts_iso,
            "type": "turn_context",
            "payload": { "cwd": session.cwd, "model": model }
        })
        .to_string(),
    );
    buf.push('\n');
    for m in msgs {
        let kind = if m.role == "assistant" {
            "output_text"
        } else {
            "input_text"
        };
        buf.push_str(
            &json!({
                "timestamp": m.ts.clone().unwrap_or_else(|| ts_iso.clone()),
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": m.role,
                    "content": [ { "type": kind, "text": m.text } ]
                }
            })
            .to_string(),
        );
        buf.push('\n');
    }
    fs::write(&path, buf)?;
    Ok(path)
}

/// Synthesize a valid v4-shaped UUID string from a seed + the current time.
/// No `rand` dependency; uniqueness comes from the nanosecond clock.
fn gen_uuid(seed: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut h: u64 = 1469598103934665603; // FNV-1a offset basis
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    let a = nanos ^ h;
    let b = h.rotate_left(32) ^ nanos.rotate_left(17);
    let hex = format!("{a:016x}{b:016x}");
    format!(
        "{}-{}-4{}-8{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_uuid_is_well_formed() {
        let u = gen_uuid("abc");
        assert_eq!(u.len(), 36);
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(
            parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
            vec![8, 4, 4, 4, 12]
        );
        assert!(u.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn extract_text_handles_string_and_blocks() {
        assert_eq!(extract_text(Some(&json!("hi"))), "hi");
        let blocks = json!([{ "type": "text", "text": "a" }, { "type": "text", "text": "b" }]);
        assert_eq!(extract_text(Some(&blocks)), "a\nb");
        assert_eq!(extract_text(None), "");
    }
}
