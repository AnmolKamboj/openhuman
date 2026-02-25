//! Programmatic skill generation for the unified skill registry.
//!
//! Supports generating both skill types:
//! - `alphahuman`: writes manifest.json + index.js to the QuickJS skills directory.
//! - `openclaw`:   writes SKILL.md or SKILL.toml to the alphahuman workspace skills directory.

use crate::unified_skills::GenerateSkillSpec;
use directories::UserDirs;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Generate an alphahuman (QuickJS) skill at `<skills_dir>/<sanitized_name>/`.
/// Returns the path of the created skill directory.
pub async fn generate_alphahuman(spec: &GenerateSkillSpec, skills_dir: &Path) -> Result<PathBuf, String> {
    let dir_name = sanitize_id(&spec.name);
    if dir_name.is_empty() {
        return Err(format!(
            "Invalid skill name '{}': must contain at least one alphanumeric character",
            spec.name
        ));
    }
    let skill_dir = skills_dir.join(&dir_name);

    tokio::fs::create_dir_all(&skill_dir)
        .await
        .map_err(|e| format!("Failed to create skill directory: {e}"))?;

    // Write manifest.json
    let manifest = serde_json::json!({
        "id": dir_name,
        "name": spec.name,
        "skill_type": "alphahuman",
        "runtime": "quickjs",
        "entry": "index.js",
        "version": "1.0.0",
        "description": spec.description,
        "auto_start": false
    });
    let manifest_path = skill_dir.join("manifest.json");
    tokio::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap())
        .await
        .map_err(|e| format!("Failed to write manifest.json: {e}"))?;

    // Write index.js
    let tool_code = spec.tool_code.as_deref().unwrap_or(
        "return { result: 'Generated skill executed successfully', args };",
    );
    let tool_fn_name = sanitize_fn_name(&spec.name);

    let index_js = build_index_js(&tool_fn_name, &spec.description, tool_code);
    tokio::fs::write(skill_dir.join("index.js"), index_js)
        .await
        .map_err(|e| format!("Failed to write index.js: {e}"))?;

    Ok(skill_dir)
}

/// Generate an openclaw (SKILL.md/TOML) skill in `~/.alphahuman/workspace/skills/<name>/`.
/// Returns the path of the created skill directory.
pub async fn generate_openclaw(spec: &GenerateSkillSpec) -> Result<PathBuf, String> {
    let dir_name = sanitize_id(&spec.name);
    if dir_name.is_empty() {
        return Err(format!(
            "Invalid skill name '{}': must contain at least one alphanumeric character",
            spec.name
        ));
    }

    let base = workspace_skills_dir()?;
    let skill_dir = base.join(&dir_name);

    tokio::fs::create_dir_all(&skill_dir)
        .await
        .map_err(|e| format!("Failed to create openclaw skill directory: {e}"))?;

    if let Some(cmd) = &spec.shell_command {
        // Structured TOML skill with a shell tool — use serde so all special
        // characters (backslashes, quotes, etc.) are correctly escaped.
        #[derive(Serialize)]
        struct SkillTomlFile {
            skill: SkillTomlMeta,
            tools: Vec<SkillTomlTool>,
        }
        #[derive(Serialize)]
        struct SkillTomlMeta {
            name: String,
            description: String,
            version: String,
        }
        #[derive(Serialize)]
        struct SkillTomlTool {
            name: String,
            description: String,
            kind: String,
            command: String,
        }

        let obj = SkillTomlFile {
            skill: SkillTomlMeta {
                name: spec.name.clone(),
                description: spec.description.clone(),
                version: "1.0.0".to_string(),
            },
            tools: vec![SkillTomlTool {
                name: sanitize_fn_name(&spec.name),
                description: spec.description.clone(),
                kind: "shell".to_string(),
                command: cmd.clone(),
            }],
        };

        let toml_content = toml::to_string(&obj)
            .map_err(|e| format!("Failed to serialize SKILL.toml: {e}"))?;
        tokio::fs::write(skill_dir.join("SKILL.toml"), toml_content)
            .await
            .map_err(|e| format!("Failed to write SKILL.toml: {e}"))?;
    } else {
        // Markdown prompt skill.
        let md_content = spec.markdown_content.as_deref().unwrap_or(&spec.description);
        let full_md = format!("# {}\n\n{}\n", spec.name, md_content);
        tokio::fs::write(skill_dir.join("SKILL.md"), full_md)
            .await
            .map_err(|e| format!("Failed to write SKILL.md: {e}"))?;
    }

    Ok(skill_dir)
}

/// Returns `~/.alphahuman/workspace/skills/`.
fn workspace_skills_dir() -> Result<PathBuf, String> {
    let dirs = UserDirs::new().ok_or("Cannot resolve home directory")?;
    Ok(dirs
        .home_dir()
        .join(".alphahuman")
        .join("workspace")
        .join("skills"))
}

/// Build a minimal but functional QuickJS skill index.js.
/// The description is JSON-serialized so newlines, backslashes, and quotes are
/// always correctly escaped for embedding in a JS string literal.
fn build_index_js(tool_fn: &str, description: &str, tool_code: &str) -> String {
    // serde_json::to_string produces a quoted, escaped JSON string literal.
    let desc_json = serde_json::to_string(description)
        .unwrap_or_else(|_| r#""unknown""#.to_string());

    format!(
        r#"// Auto-generated alphahuman skill
const tools = [
  {{
    name: "{tool_fn}",
    description: {desc},
    inputSchema: {{
      type: "object",
      properties: {{
        args: {{ type: "object", description: "Optional arguments" }}
      }}
    }}
  }}
];

async function init() {{}}

async function start() {{}}

async function {tool_fn}(args) {{
  {tool_code}
}}
"#,
        tool_fn = tool_fn,
        desc = desc_json,
        tool_code = tool_code,
    )
}

/// Convert a display name to a filesystem-safe id (lowercase, hyphens).
fn sanitize_id(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert a display name to a valid JS function name (lowercase, underscores).
/// Ensures the result never starts with a digit by prepending `_` when needed.
fn sanitize_fn_name(name: &str) -> String {
    let joined = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    // JS identifiers must not start with a digit.
    if joined.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{joined}")
    } else {
        joined
    }
}
