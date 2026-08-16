//! Host-side vault capture: "save this to vault" writes a note into the
//! configured folder memory source (smriti) and pushes it to GitHub.
//!
//! Same contract as reminders: channel turns cannot be trusted to call
//! `file_write` (sandbox is the OpenHuman workspace, not the vault) or to
//! run git. The user typed the save; the host does it.

use crate::openhuman::config::Config;
use crate::openhuman::memory::sources::types::SourceKind;
use chrono::Local;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A fact the user asked to persist in the vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultSaveRequest {
    pub fact: String,
    pub person: Option<String>,
}

/// Parse "save this to vault / memory vault / smriti" and pick the fact.
///
/// The fact is whatever follows the command, or the last prior user turn
/// when this turn is only the command ("save this to the vault").
pub fn parse_vault_save(current: &str, recent_user_texts: &[&str]) -> Option<VaultSaveRequest> {
    let fact = extract_save_fact(current, recent_user_texts)?;
    let person = extract_person_name(&fact);
    Some(VaultSaveRequest { fact, person })
}

/// Write the fact into the smriti vault and push to the existing GitHub remote.
///
/// Returns a prompt block so the model confirms instead of inventing a save.
pub fn save_requested_fact(
    config: &Config,
    current: &str,
    recent_user_texts: &[&str],
) -> Option<String> {
    let request = parse_vault_save(current, recent_user_texts)?;
    let Some(vault) = folder_source_root(config) else {
        return Some(
            "[VAULT SAVE FAILED — no folder memory source is configured, so there is no smriti \
             vault to write. Tell the user plainly that it was not saved.]\n"
                .to_string(),
        );
    };
    match persist_and_push(&vault, &request) {
        Ok(written) => {
            tracing::info!(
                files = written.len(),
                person = request.person.as_deref().unwrap_or(""),
                "[vault_save] wrote and pushed vault note"
            );
            Some(format!(
                "[VAULT SAVED — the host already wrote this into the smriti vault and pushed \
                 it to GitHub. Confirm in one short line. Name the file(s). Do not call \
                 file_write or git; it is done.]\n\nFact: {}\nFiles: {}\n",
                request.fact,
                written.join(", ")
            ))
        }
        Err(err) => {
            tracing::warn!(error = %err, "[vault_save] failed to save or push");
            Some(format!(
                "[VAULT SAVE FAILED — the host could not write or push the note: {err}. \
                 Tell the user plainly that it was not saved. Do not claim it is in the vault.]\n"
            ))
        }
    }
}

const SAVE_COMMANDS: &[&str] = &[
    "save this to the memory vault",
    "save this to memory vault",
    "save this to the smriti vault",
    "save this to smriti vault",
    "save this to the vault",
    "save this to vault",
    "save this to smriti",
    "save to the memory vault",
    "save to memory vault",
    "save to the vault",
    "save to vault",
    "save to smriti",
];

fn extract_save_fact(current: &str, recent_user_texts: &[&str]) -> Option<String> {
    let lowered = current.to_ascii_lowercase();
    let cmd = SAVE_COMMANDS.iter().find(|c| lowered.contains(*c))?;
    let start = lowered.find(cmd)?;
    let after = current[start + cmd.len()..]
        .trim()
        .trim_start_matches([':', '-', ',', '.'])
        .trim()
        .trim_matches('"')
        .trim();

    if has_named_fact(after) {
        return Some(after.to_string());
    }

    for text in recent_user_texts.iter().rev() {
        if text.trim() == current.trim() {
            continue;
        }
        let prior_lower = text.to_ascii_lowercase();
        if SAVE_COMMANDS.iter().any(|c| prior_lower.contains(c)) {
            continue;
        }
        let prior = text.trim();
        if has_named_fact(prior) {
            return Some(prior.to_string());
        }
    }
    None
}

fn has_named_fact(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    const EMPTIES: &[&str] = &["this", "it", "that", "please", "now", "ok", "okay"];
    !EMPTIES.contains(&t.to_ascii_lowercase().as_str())
}

/// Pull a person name out of common phrasings without a second LLM call.
fn extract_person_name(fact: &str) -> Option<String> {
    let lowered = fact.to_ascii_lowercase();
    for marker in ["her name is ", "his name is ", "their name is ", "named "] {
        if let Some(idx) = lowered.find(marker) {
            let rest = fact[idx + marker.len()..].trim();
            if let Some(name) = first_proper_name(rest) {
                return Some(name);
            }
        }
    }
    None
}

fn first_proper_name(text: &str) -> Option<String> {
    let token = text
        .split(|c: char| !c.is_alphabetic() && c != '-' && c != '\'')
        .find(|w| !w.is_empty())?;
    if token.chars().count() < 2 {
        return None;
    }
    let lower = token.to_ascii_lowercase();
    const STOP: &[&str] = &[
        "a", "an", "the", "my", "and", "or", "is", "was", "i", "me", "we", "she", "he", "they",
        "her", "his", "their",
    ];
    if STOP.contains(&lower.as_str()) {
        return None;
    }
    Some(title_case(token))
}

fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = first.to_uppercase().collect::<String>();
    out.extend(chars.flat_map(|c| c.to_lowercase()));
    out
}

fn folder_source_root(config: &Config) -> Option<PathBuf> {
    config.memory_sources.iter().find_map(|src| {
        if src.enabled && src.kind == SourceKind::Folder {
            src.path
                .as_deref()
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
        } else {
            None
        }
    })
}

fn persist_and_push(vault: &Path, request: &VaultSaveRequest) -> Result<Vec<String>, String> {
    if !vault.is_dir() {
        return Err("smriti vault folder is missing".into());
    }
    let today = Local::now().date_naive();
    let date = today.format("%Y-%m-%d").to_string();
    let mut written = Vec::new();

    let daily_rel = format!("Daily/{date}.md");
    append_daily(vault, &daily_rel, &request.fact, request.person.as_deref())?;
    written.push(daily_rel);

    if let Some(person) = request.person.as_deref() {
        let person_rel = format!("People/{person}.md");
        upsert_person(vault, &person_rel, person, &request.fact, &date)?;
        written.push(person_rel);
        let anmol_rel = "People/Anmol.md";
        if vault.join(anmol_rel).is_file() {
            append_anmol_link(vault, anmol_rel, person, &request.fact)?;
            written.push(anmol_rel.to_string());
        }
    }

    push_vault_files(vault, &written, &request.fact)?;
    Ok(written)
}

fn append_daily(
    vault: &Path,
    rel: &str,
    fact: &str,
    person: Option<&str>,
) -> Result<(), String> {
    let path = resolve_vault_file(vault, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create Daily/: {e}"))?;
    }
    let line = match person {
        Some(name) => format!("- {fact} [[{name}]]\n"),
        None => format!("- {fact}\n"),
    };
    if path.is_file() {
        let mut body = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
        if !body.ends_with('\n') {
            body.push('\n');
        }
        if !body.contains("## Captured") {
            body.push_str("\n## Captured\n\n");
        }
        body.push_str(&line);
        std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))?;
    } else {
        let date = Local::now().format("%Y-%m-%d");
        let body = format!(
            "---\ntags: [daily]\ndate: {date}\ntype: daily\n---\n\n# {date}\n\n## Captured\n\n{line}"
        );
        std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))?;
    }
    Ok(())
}

fn upsert_person(
    vault: &Path,
    rel: &str,
    name: &str,
    fact: &str,
    date: &str,
) -> Result<(), String> {
    let path = resolve_vault_file(vault, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create People/: {e}"))?;
    }
    if path.is_file() {
        let mut body = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
        if body.contains(fact) {
            return Ok(());
        }
        if !body.ends_with('\n') {
            body.push('\n');
        }
        if !body.contains("## Key facts") {
            body.push_str("\n## Key facts\n\n");
        }
        body.push_str(&format!("- {fact}\n"));
        std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))?;
    } else {
        let body = format!(
            "---\ntags: [person]\ndate: {date}\ntype: person\n---\n\n# {name}\n\n## Key facts\n- {fact}\n\n## Appears in\n- [[{date}]]\n"
        );
        std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))?;
    }
    Ok(())
}

fn append_anmol_link(vault: &Path, rel: &str, person: &str, fact: &str) -> Result<(), String> {
    let path = resolve_vault_file(vault, rel)?;
    let mut body = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
    let needle = format!("[[{person}]]");
    if body.contains(&needle) && body.contains(fact) {
        return Ok(());
    }
    if !body.ends_with('\n') {
        body.push('\n');
    }
    if !body.contains("## Key facts") {
        body.push_str("\n## Key facts\n\n");
    }
    body.push_str(&format!("- {fact} [[{person}]]\n"));
    std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))
}

fn resolve_vault_file(vault: &Path, rel: &str) -> Result<PathBuf, String> {
    let normalized = rel.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains("..")
        || normalized.contains('\0')
    {
        return Err("vault path must stay inside the vault".into());
    }
    let mut out = vault.to_path_buf();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("vault path must stay inside the vault".into());
        }
        out.push(part);
    }
    Ok(out)
}

fn push_vault_files(vault: &Path, rels: &[String], fact: &str) -> Result<(), String> {
    if !vault.join(".git").exists() {
        return Err("vault is not a git repo".into());
    }
    ensure_git_identity(vault);
    for rel in rels {
        run_git(vault, &["add", "--", rel])?;
    }
    let status = run_git(vault, &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        return Ok(());
    }
    let summary: String = fact.chars().take(72).collect();
    let message = format!("Smriti: {summary}");
    run_git(vault, &["commit", "-m", &message])?;

    let creds = load_vault_git_creds();
    let env = git_quiet_env();
    let remote = creds
        .as_ref()
        .map(|c| c.auth_url())
        .unwrap_or_else(|| "origin".to_string());
    let token = creds.as_ref().map(|c| c.token.as_str());
    let _ = run_git_env(vault, &["pull", "--rebase", &remote, "HEAD"], env.clone(), token);
    run_git_env(vault, &["push", &remote, "HEAD"], env, token)?;
    Ok(())
}

struct VaultGitCreds {
    remote: String,
    token: String,
}

impl VaultGitCreds {
    fn auth_url(&self) -> String {
        if self.remote.starts_with("https://") && !self.token.is_empty() {
            self.remote.replacen("https://", &format!("https://{}@", self.token), 1)
        } else {
            "origin".to_string()
        }
    }
}

fn load_vault_git_creds() -> Option<VaultGitCreds> {
    let mut remote = std::env::var("VAULT_GIT_REMOTE").ok().unwrap_or_default();
    let mut token = std::env::var("VAULT_GIT_TOKEN").ok().unwrap_or_default();
    if remote.is_empty() || token.is_empty() {
        if let Some(file) = read_smriti_env() {
            if remote.is_empty() {
                remote = file.get("VAULT_GIT_REMOTE").cloned().unwrap_or_default();
            }
            if token.is_empty() {
                token = file.get("VAULT_GIT_TOKEN").cloned().unwrap_or_default();
            }
        }
    }
    remote = remote.trim().to_string();
    token = token.trim().to_string();
    if remote.is_empty() {
        return None;
    }
    Some(VaultGitCreds { remote, token })
}

fn smriti_env_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("SMRITI_ENV_FILE") {
        out.push(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("SMRITI_DIR") {
        out.push(PathBuf::from(p).join(".env"));
    }
    out.push(PathBuf::from(r"D:\Projects\smriti\.env"));
    out
}

fn read_smriti_env() -> Option<HashMap<String, String>> {
    for path in smriti_env_paths() {
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut map = HashMap::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
        if !map.is_empty() {
            return Some(map);
        }
    }
    None
}

fn ensure_git_identity(vault: &Path) {
    let name = run_git(vault, &["config", "user.name"]).unwrap_or_default();
    if name.trim().is_empty() {
        let _ = run_git(vault, &["config", "user.name", "OpenHuman"]);
        let _ = run_git(vault, &["config", "user.email", "openhuman@local"]);
    }
}

fn git_quiet_env() -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());
    env.insert("GIT_ASKPASS".into(), "echo".into());
    env
}

fn run_git(vault: &Path, args: &[&str]) -> Result<String, String> {
    run_git_env(vault, args, git_quiet_env(), None)
}

fn run_git_env(
    vault: &Path,
    args: &[&str],
    env: HashMap<String, String>,
    token: Option<&str>,
) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(vault);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    for (k, v) in &env {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("git {} failed to start: {e}", args.first().unwrap_or(&"git")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    Err(redact(&combined, token))
}

fn redact(text: &str, token: Option<&str>) -> String {
    let mut out = text.replace('\r', " ");
    if let Some(token) = token {
        if !token.is_empty() {
            out = out.replace(token, "***");
        }
    }
    out.chars().take(400).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_plus_fact_in_one_turn() {
        let parsed = parse_vault_save(
            "save this to vault: I have a girlfriend and her name is Vivian",
            &[],
        )
        .unwrap();
        assert_eq!(
            parsed.fact,
            "I have a girlfriend and her name is Vivian"
        );
        assert_eq!(parsed.person.as_deref(), Some("Vivian"));
    }

    #[test]
    fn command_alone_reuses_the_previous_fact() {
        let prior = ["I have a girlfriend and her name is Vivian"];
        let parsed = parse_vault_save("save this to the memory vault", &prior).unwrap();
        assert_eq!(parsed.person.as_deref(), Some("Vivian"));
    }

    #[test]
    fn ignores_unrelated_turns() {
        assert!(parse_vault_save("what's the weather", &[]).is_none());
        assert!(parse_vault_save("save this to vault", &["ok"]).is_none());
    }

    #[test]
    fn named_marker_is_required_for_a_person_file() {
        let parsed = parse_vault_save("save this to smriti: I like oat milk", &[]).unwrap();
        assert!(parsed.person.is_none());
        assert_eq!(parsed.fact, "I like oat milk");
    }

    #[test]
    fn vault_paths_cannot_escape() {
        let root = Path::new("C:/vault");
        assert!(resolve_vault_file(root, "../secret").is_err());
        assert!(resolve_vault_file(root, "People/Vivian.md").is_ok());
    }

    #[test]
    fn redact_strips_the_token_from_errors() {
        assert_eq!(redact("fatal: ghp_secret boom", Some("ghp_secret")), "fatal: *** boom");
    }
}
