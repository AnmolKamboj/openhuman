//! Host-side vault capture: "save this to vault" structures a note the same
//! way Jarvis does (`docs/memory_guidelines.md`) and pushes it to GitHub.
//!
//! Channel turns cannot write the Obsidian folder (`file_write` is sandboxed
//! to the OpenHuman workspace) and cannot run git. The user typed the save;
//! the host does it. After the write, the folder memory source is synced so
//! OpenHuman's own embedder (NVIDIA 1024-dim by default) picks the new note
//! up for the graph and `memory_recall`.

use crate::openhuman::config::Config;
use crate::openhuman::memory::sources::types::SourceKind;
use chrono::Local;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const SELF_TITLE: &str = "Anmol";
const SELF_PATH: &str = "People/Anmol.md";

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

/// Write the fact into the smriti vault (Jarvis rules) and push to GitHub.
///
/// Returns a prompt block so the model confirms instead of inventing a save.
pub async fn save_requested_fact(
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
    match persist_and_push(config, &vault, &request).await {
        Ok(written) => {
            trigger_folder_ingest(config);
            tracing::info!(
                files = written.len(),
                person = request.person.as_deref().unwrap_or(""),
                "[vault_save] wrote, pushed, and queued folder ingest"
            );
            Some(format!(
                "[VAULT SAVED — the host already structured this with the Jarvis vault rules, \
                 wrote it into the smriti vault, pushed GitHub, and queued OpenHuman folder \
                 ingest so the memory graph can embed it. Confirm in one short line. Name \
                 the file(s). Do not call file_write or git; it is done.]\n\nFact: {}\nFiles: {}\n",
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

async fn persist_and_push(
    config: &Config,
    vault: &Path,
    request: &VaultSaveRequest,
) -> Result<Vec<String>, String> {
    if !vault.is_dir() {
        return Err("smriti vault folder is missing".into());
    }
    let notes = list_notes(vault)?;
    let written = match structure_with_llm(config, &notes, &request.fact).await {
        Ok(dump) if !dump.notes.is_empty() => apply_structured_dump(vault, &notes, dump)?,
        Ok(_) => {
            tracing::warn!("[vault_save] LLM returned no notes; using heuristic fallback");
            heuristic_write(vault, request)?
        }
        Err(err) => {
            tracing::warn!(error = %err, "[vault_save] LLM structure failed; using heuristic fallback");
            heuristic_write(vault, request)?
        }
    };
    if written.is_empty() {
        return Err("nothing was written to the vault".into());
    }
    push_vault_files(vault, &written, &request.fact)?;
    Ok(written)
}

#[derive(Debug, Deserialize)]
struct StructuredDump {
    #[serde(default)]
    notes: Vec<StructuredNote>,
    #[serde(default)]
    summary: String,
}

#[derive(Debug, Deserialize)]
struct StructuredNote {
    relative_path: String,
    content: String,
    #[serde(default)]
    hub_updates: Vec<HubUpdate>,
}

#[derive(Debug, Deserialize)]
struct HubUpdate {
    #[serde(rename = "type")]
    hub_type: String,
    name: String,
    #[serde(default)]
    context: String,
}

#[derive(Debug, Clone)]
struct VaultNote {
    title: String,
    content: String,
    relative_path: String,
}

async fn structure_with_llm(
    config: &Config,
    notes: &[VaultNote],
    fact: &str,
) -> Result<StructuredDump, String> {
    let (model, model_id) =
        crate::openhuman::inference::provider::create_chat_model_with_model_id(
            "memory", config, 0.2,
        )
        .map_err(|e| format!("structure model unavailable: {e:#}"))?;
    let prompt = build_structure_prompt(notes, fact);
    tracing::info!(model = %model_id, "[vault_save] structuring note with Jarvis rules");
    use tinyagents::harness::message::Message;
    use tinyagents::harness::model::ModelRequest;
    let text = model
        .invoke(
            &(),
            ModelRequest::new(vec![
                Message::system(
                    "You structure personal memory notes for an Obsidian vault. \
                     Follow the guidelines exactly. Return valid JSON only.",
                ),
                Message::user(prompt),
            ]),
        )
        .await
        .map_err(|e| format!("structure call failed: {e:#}"))?
        .text();
    parse_structure_json(&text)
}

fn build_structure_prompt(notes: &[VaultNote], fact: &str) -> String {
    let today = Local::now().format("%Y-%m-%d");
    let titles: Vec<&str> = notes.iter().map(|n| n.title.as_str()).collect();
    let related = search_related(notes, fact, 8);
    let related_summary = if related.is_empty() {
        "None".to_string()
    } else {
        related
            .iter()
            .map(|n| {
                let snippet: String = n.content.chars().take(800).collect();
                format!("File: {}\n{snippet}", n.relative_path)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    format!(
        "GUIDELINES:\n{}\n\n\
TODAY'S DATE: {today}\n\n\
EXISTING NOTE TITLES (one entity = one note; reuse these exact names, never invent variants):\n{}\n\n\
EXISTING RELATED NOTES:\n{related_summary}\n\n\
NEW RAW INPUT:\n{fact}\n\n\
FACT DISCIPLINE (highest priority, overrides style guidance):\n\
- Only NEW RAW INPUT may contribute new facts. EXISTING RELATED NOTES are for \
  dedup and linking only - never copy their facts into a note you write.\n\
- One fact, one home: a fact goes only into the note of the entity it explicitly \
  concerns. If the input mentions two entities separately, each note gets only \
  its own facts. Never blend separate events into one shared narrative.\n\
- Never infer relationships that were not stated. If it was not said, it does not go in the vault.\n\
- Record plainly what was said. No editorializing, no combining separate events into a story.\n\
- Use plain hyphens, never em dashes, in note content.\n\
- There is exactly ONE note about the user: `{SELF_PATH}`. Never create Myself.md, Me.md, \
  or any other self-variant. Link to the user as [[{SELF_TITLE}]] only.\n\
- A note never links to itself. Daily notes are a chronological log under Daily/{today}.md.\n\
- When a new person, place, trait, or idea appears, include a hub_updates entry.\n\n\
Return JSON with this shape:\n\
{{\n\
  \"notes\": [\n\
    {{\n\
      \"relative_path\": \"Daily/{today}.md or People/Name.md etc\",\n\
      \"content\": \"full markdown note content with frontmatter\",\n\
      \"hub_updates\": [\n\
        {{\"type\": \"person|place|trait|idea\", \"name\": \"Name\", \"context\": \"why it appears\"}}\n\
      ]\n\
    }}\n\
  ],\n\
  \"summary\": \"one sentence for the user\"\n\
}}\n",
        load_guidelines(),
        if titles.is_empty() {
            "None".to_string()
        } else {
            titles.join(", ")
        }
    )
}

fn load_guidelines() -> String {
    for path in guidelines_paths() {
        if let Ok(body) = std::fs::read_to_string(path) {
            if !body.trim().is_empty() {
                return body;
            }
        }
    }
    JARVIS_GUIDELINES.to_string()
}

fn guidelines_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("SMRITI_GUIDELINES") {
        out.push(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("SMRITI_DIR") {
        out.push(PathBuf::from(p).join("docs").join("memory_guidelines.md"));
    }
    out.push(PathBuf::from(r"D:\Projects\smriti\docs\memory_guidelines.md"));
    out
}

const JARVIS_GUIDELINES: &str = r#"# Memory Guidelines — Personal Memory & Second Brain

## Me (the user)
- I am Anmol Kamboj. There is exactly ONE note about me: `People/Anmol.md`.
- When I share facts about myself, update `People/Anmol.md`. Never create `Myself.md`, `Me.md`, or any other self-variant.
- Link to me as `[[Anmol]]` only.
- A note never links to itself.

## Fact discipline
- Only the new raw input may contribute new facts.
- One fact, one home. Never infer relationships that were not stated.
- Record plainly what was said. Use plain hyphens, never em dashes.

## File types
- Person: `People/Name.md`
- Place: `Places/Name.md`
- Trait: `Traits/Name.md`
- Idea: `Ideas/Name.md`
- Daily: `Daily/YYYY-MM-DD.md`
- Event: `YYYY - Title.md` at vault root

## Linking
- One entity = one note. Reuse existing titles. Every person, place, trait, or idea gets a hub note.
- Wrap genuine mentions in `[[double brackets]]`. Update hubs in both directions.
"#;

fn parse_structure_json(text: &str) -> Result<StructuredDump, String> {
    let cleaned = strip_json_fence(text);
    if let Ok(dump) = serde_json::from_str::<StructuredDump>(cleaned) {
        return Ok(dump);
    }
    let start = cleaned.find('{').ok_or_else(|| "invalid structure JSON: no object".to_string())?;
    let end = cleaned
        .rfind('}')
        .ok_or_else(|| "invalid structure JSON: no object".to_string())?;
    serde_json::from_str(&cleaned[start..=end])
        .map_err(|e| format!("invalid structure JSON: {e}"))
}

fn strip_json_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest)
        .trim_start_matches('\r')
        .trim_start_matches('\n');
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

fn apply_structured_dump(
    vault: &Path,
    existing: &[VaultNote],
    dump: StructuredDump,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    let mut notes = existing.to_vec();
    for note in dump.notes {
        let rel = canonicalize_relative_path(&note.relative_path)?;
        let content = canonicalize_links(&notes, &sanitize_dashes(&note.content));
        let source_title = Path::new(&rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let (final_rel, final_title) = if rel.starts_with("Daily/") {
            write_note(vault, &rel, &content)?;
            (rel, source_title)
        } else if let Some(similar) = find_similar(&notes, &source_title) {
            if similar.relative_path != rel {
                let body = strip_frontmatter(&content);
                append_to_note(vault, &similar.relative_path, &body)?;
                (similar.relative_path.clone(), similar.title.clone())
            } else {
                write_note(vault, &rel, &content)?;
                (rel, source_title)
            }
        } else {
            write_note(vault, &rel, &content)?;
            (rel, source_title)
        };

        if !written.contains(&final_rel) {
            written.push(final_rel.clone());
        }
        refresh_note_cache(&mut notes, vault, &final_rel)?;
        for hub in apply_hub_updates(vault, &notes, &note.hub_updates, &final_title)? {
            if !written.contains(&hub) {
                written.push(hub.clone());
            }
            refresh_note_cache(&mut notes, vault, &hub)?;
        }
    }
    let _ = dump.summary;
    Ok(written)
}

fn refresh_note_cache(notes: &mut Vec<VaultNote>, vault: &Path, rel: &str) -> Result<(), String> {
    if let Some(note) = read_note(vault, rel)? {
        if let Some(existing) = notes.iter_mut().find(|n| n.relative_path == rel) {
            *existing = note;
        } else {
            notes.push(note);
        }
    }
    Ok(())
}

fn apply_hub_updates(
    vault: &Path,
    notes: &[VaultNote],
    hubs: &[HubUpdate],
    source_title: &str,
) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for hub in hubs {
        let name = canonicalize_self_title(&hub.name);
        if name.eq_ignore_ascii_case(source_title) {
            continue;
        }
        let hub_name = find_similar(notes, &name)
            .map(|n| n.title.clone())
            .unwrap_or(name);
        let rel = ensure_hub_note(vault, notes, &hub.hub_type, &hub_name, &hub.context, source_title)?;
        written.push(rel);
    }
    Ok(written)
}

fn ensure_hub_note(
    vault: &Path,
    notes: &[VaultNote],
    hub_type: &str,
    name: &str,
    context: &str,
    source_note: &str,
) -> Result<String, String> {
    let folder = match hub_type.trim().to_ascii_lowercase().as_str() {
        "person" => "People",
        "place" => "Places",
        "trait" => "Traits",
        _ => "Ideas",
    };
    let safe = safe_filename(name);
    let mut relative_path = format!("{folder}/{safe}.md");
    if let Some(titled) = note_by_title(notes, name) {
        relative_path = titled.relative_path.clone();
    }
    let context = sanitize_dashes(context);
    if let Some(existing) = read_note(vault, &relative_path)? {
        let needle = format!("[[{source_note}]]");
        if !existing.content.contains(&needle) {
            let section = if existing.content.contains("## Appears in") {
                format!("- [[{source_note}]] - {context}")
            } else {
                format!("## Appears in\n- [[{source_note}]] - {context}")
            };
            append_to_note(vault, &relative_path, &section)?;
        }
        return Ok(relative_path);
    }
    let body = format!(
        "---\ntags: [{hub_type}]\n---\n\n# {name}\n\n{context}\n\n## Appears in\n- [[{source_note}]] - {context}\n\n## What I know so far\n{context}\n"
    );
    write_note(vault, &relative_path, &body)?;
    Ok(relative_path)
}

fn heuristic_write(vault: &Path, request: &VaultSaveRequest) -> Result<Vec<String>, String> {
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
        if vault.join(SELF_PATH).is_file() {
            append_anmol_link(vault, SELF_PATH, person, &request.fact)?;
            written.push(SELF_PATH.to_string());
        }
    }

    Ok(written)
}

fn append_daily(vault: &Path, rel: &str, fact: &str, person: Option<&str>) -> Result<(), String> {
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

fn list_notes(vault: &Path) -> Result<Vec<VaultNote>, String> {
    let mut notes = Vec::new();
    let mut stack = vec![vault.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("scan vault: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("scan vault: {e}"))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !name.ends_with(".md") || name.starts_with('_') {
                continue;
            }
            let rel = path
                .strip_prefix(vault)
                .map_err(|_| "vault path escaped".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            notes.push(VaultNote {
                title: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
                content,
                relative_path: rel,
            });
        }
    }
    Ok(notes)
}

fn read_note(vault: &Path, rel: &str) -> Result<Option<VaultNote>, String> {
    let path = resolve_vault_file(vault, rel)?;
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
    Ok(Some(VaultNote {
        title: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        content,
        relative_path: rel.replace('\\', "/"),
    }))
}

fn write_note(vault: &Path, rel: &str, content: &str) -> Result<(), String> {
    let path = resolve_vault_file(vault, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut body = content.trim_end().to_string();
    body.push('\n');
    std::fs::write(&path, body).map_err(|e| format!("write {rel}: {e}"))
}

fn append_to_note(vault: &Path, rel: &str, section: &str) -> Result<(), String> {
    if let Some(existing) = read_note(vault, rel)? {
        let mut body = existing.content;
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push('\n');
        body.push_str(section.trim());
        body.push('\n');
        write_note(vault, rel, &body)
    } else {
        write_note(vault, rel, section)
    }
}

fn note_by_title<'a>(notes: &'a [VaultNote], title: &str) -> Option<&'a VaultNote> {
    let lowered = title.trim().to_ascii_lowercase();
    notes.iter().find(|n| n.title.eq_ignore_ascii_case(&lowered))
}

fn find_similar<'a>(notes: &'a [VaultNote], name: &str) -> Option<&'a VaultNote> {
    let target = normalize_title(name);
    if target.is_empty() {
        return None;
    }
    let mut best_score = 0.0_f64;
    let mut best: Option<&VaultNote> = None;
    for note in notes {
        let candidate = normalize_title(&note.title);
        if candidate.is_empty() {
            continue;
        }
        if candidate == target {
            return Some(note);
        }
        let mut score = sequence_ratio(&target, &candidate);
        let (shorter, longer) = if target.len() <= candidate.len() {
            (target.as_str(), candidate.as_str())
        } else {
            (candidate.as_str(), target.as_str())
        };
        if shorter.len() >= 4 && longer.contains(shorter) {
            score = score.max(0.9);
        }
        if score > best_score {
            best_score = score;
            best = Some(note);
        }
    }
    if best_score >= 0.88 {
        best
    } else {
        None
    }
}

fn search_related<'a>(notes: &'a [VaultNote], query: &str, limit: usize) -> Vec<&'a VaultNote> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(|t| t.to_ascii_lowercase())
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &VaultNote)> = notes
        .iter()
        .filter_map(|note| {
            let hay = format!("{}\n{}", note.title, note.content).to_ascii_lowercase();
            let score: usize = terms.iter().map(|t| hay.matches(t.as_str()).count()).sum();
            (score > 0).then_some((score, note))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(limit).map(|(_, n)| n).collect()
}

fn canonicalize_links(notes: &[VaultNote], content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            out.push_str("[[");
            out.push_str(rest);
            return out;
        };
        let inner = &rest[..end];
        rest = &rest[end + 2..];
        let (target, alias) = match inner.split_once('|') {
            Some((t, a)) => (t.trim(), Some(a)),
            None => (inner.trim(), None),
        };
        let mut canonical = canonicalize_self_title(target);
        if let Some(exact) = note_by_title(notes, &canonical) {
            canonical = exact.title.clone();
        } else if let Some(similar) = find_similar(notes, &canonical) {
            canonical = similar.title.clone();
        }
        out.push_str("[[");
        out.push_str(&canonical);
        if let Some(alias) = alias {
            out.push('|');
            out.push_str(alias);
        }
        out.push_str("]]");
    }
    out.push_str(rest);
    out
}

fn canonicalize_relative_path(rel: &str) -> Result<String, String> {
    let normalized = rel.replace('\\', "/").trim().trim_start_matches('/').to_string();
    let path = Path::new(&normalized);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if is_forbidden_self_title(&stem) {
        return Ok(SELF_PATH.to_string());
    }
    resolve_vault_file(Path::new("C:/vault"), &normalized)?;
    if !normalized.ends_with(".md") {
        return Err("vault note must be a .md file".into());
    }
    Ok(normalized)
}

fn canonicalize_self_title(title: &str) -> String {
    if is_forbidden_self_title(title) {
        SELF_TITLE.to_string()
    } else {
        title.trim().to_string()
    }
}

fn is_forbidden_self_title(title: &str) -> bool {
    matches!(
        title.trim().to_ascii_lowercase().as_str(),
        "myself" | "me" | "i" | "user" | "the user"
    )
}

fn sanitize_dashes(text: &str) -> String {
    text.replace('\u{2014}', "-")
        .replace('\u{2013}', "-")
        .replace('\u{fffd}', "-")
}

fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim().to_string();
        }
    }
    content.trim().to_string()
}

fn normalize_title(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn sequence_ratio(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let lcs = lcs_len(a.as_bytes(), b.as_bytes());
    (2.0 * lcs as f64) / (a.len() + b.len()) as f64
}

fn lcs_len(a: &[u8], b: &[u8]) -> usize {
    let (a, b) = if a.len() > 64 || b.len() > 64 {
        (
            &a[..a.len().min(64)],
            &b[..b.len().min(64)],
        )
    } else {
        (a, b)
    };
    let mut prev = vec![0usize; b.len() + 1];
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = if ca == cb {
                prev[j] + 1
            } else {
                prev[j + 1].max(cur[j])
            };
        }
        if i + 1 < a.len() {
            std::mem::swap(&mut prev, &mut cur);
            cur.fill(0);
        }
    }
    cur[b.len()]
}

fn safe_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Untitled".into()
    } else {
        trimmed.to_string()
    }
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

fn trigger_folder_ingest(config: &Config) {
    let source_ids: Vec<String> = config
        .memory_sources
        .iter()
        .filter(|src| src.enabled && src.kind == SourceKind::Folder)
        .map(|src| src.id.clone())
        .collect();
    if source_ids.is_empty() {
        return;
    }
    tokio::spawn(async move {
        for source_id in source_ids {
            match crate::openhuman::memory::sources::rpc::sync_rpc(
                crate::openhuman::memory::sources::rpc::SyncRequest {
                    source_id: source_id.clone(),
                },
            )
            .await
            {
                Ok(_) => tracing::info!(source_id = %source_id, "[vault_save] folder ingest queued"),
                Err(err) => tracing::warn!(
                    source_id = %source_id,
                    error = %err,
                    "[vault_save] folder ingest failed"
                ),
            }
        }
    });
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
            self.remote
                .replacen("https://", &format!("https://{}@", self.token), 1)
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
        assert_eq!(
            redact("fatal: ghp_secret boom", Some("ghp_secret")),
            "fatal: *** boom"
        );
    }

    #[test]
    fn myself_notes_are_rewritten_to_anmol() {
        assert_eq!(
            canonicalize_relative_path("People/Myself.md").unwrap(),
            SELF_PATH
        );
        assert_eq!(canonicalize_self_title("Me"), SELF_TITLE);
    }

    #[test]
    fn find_similar_reuses_an_existing_title() {
        let notes = vec![VaultNote {
            title: "Vivian".into(),
            content: String::new(),
            relative_path: "People/Vivian.md".into(),
        }];
        assert_eq!(
            find_similar(&notes, "vivian").map(|n| n.relative_path.as_str()),
            Some("People/Vivian.md")
        );
    }

    #[test]
    fn canonicalize_links_rewrites_self_and_aliases() {
        let notes = vec![VaultNote {
            title: "Vivian".into(),
            content: String::new(),
            relative_path: "People/Vivian.md".into(),
        }];
        let out = canonicalize_links(&notes, "saw [[Me]] with [[vivian]]");
        assert_eq!(out, "saw [[Anmol]] with [[Vivian]]");
    }

    #[test]
    fn parse_structure_json_accepts_fenced_payload() {
        let dump = parse_structure_json(
            "```json\n{\"notes\":[{\"relative_path\":\"Daily/2026-08-15.md\",\"content\":\"# x\"}],\"summary\":\"ok\"}\n```",
        )
        .unwrap();
        assert_eq!(dump.notes[0].relative_path, "Daily/2026-08-15.md");
        assert_eq!(dump.summary, "ok");
    }

    #[test]
    fn heuristic_writes_person_daily_and_anmol_hub() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path();
        std::fs::create_dir_all(vault.join("People")).unwrap();
        std::fs::write(vault.join(SELF_PATH), "# Anmol\n\n## Key facts\n").unwrap();
        let written = heuristic_write(
            vault,
            &VaultSaveRequest {
                fact: "I have a girlfriend and her name is Vivian".into(),
                person: Some("Vivian".into()),
            },
        )
        .unwrap();
        assert!(written.iter().any(|p| p.starts_with("Daily/")));
        assert!(written.iter().any(|p| p == "People/Vivian.md"));
        assert!(written.iter().any(|p| p == SELF_PATH));
        let vivian = std::fs::read_to_string(vault.join("People/Vivian.md")).unwrap();
        assert!(vivian.contains("Vivian"));
        let anmol = std::fs::read_to_string(vault.join(SELF_PATH)).unwrap();
        assert!(anmol.contains("[[Vivian]]"));
    }
}
