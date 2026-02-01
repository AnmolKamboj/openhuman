//! Skill data I/O commands.
//!
//! Provides filesystem operations for skill data directories,
//! used by the reverse RPC handler in the frontend.

use std::path::PathBuf;

/// Absolute path to the skills working directory (cwd for `python -m skills.xxx`).
/// In dev: project root's `skills/` (submodule). In prod: `~/.alphahuman/skills/`.
///
/// When running via `tauri dev`, the Rust binary's cwd is `src-tauri/`,
/// so we also check `../skills` (the project root's submodule).
#[tauri::command]
pub async fn skill_cwd() -> Result<String, String> {
    let current = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?;

    // Check: cwd/skills (if running from project root)
    let dev_skills = current.join("skills");
    if dev_skills.join("skills").exists() {
        return dev_skills
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize skills dir: {}", e))?
            .into_os_string()
            .into_string()
            .map_err(|_| "Invalid path".to_string());
    }

    // Check: ../skills (if running from src-tauri/ via `tauri dev`)
    if let Some(parent) = current.parent() {
        let parent_skills = parent.join("skills");
        if parent_skills.join("skills").exists() {
            return parent_skills
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize skills dir: {}", e))?
                .into_os_string()
                .into_string()
                .map_err(|_| "Invalid path".to_string());
        }
    }

    // Production fallback: ~/.alphahuman/skills/
    let data_dir = crate::ai::encryption::get_data_dir()?;
    let skills_dir = data_dir.join("skills");
    std::fs::create_dir_all(&skills_dir)
        .map_err(|e| format!("Failed to create skills dir: {}", e))?;
    skills_dir
        .into_os_string()
        .into_string()
        .map_err(|_| "Invalid path".to_string())
}

/// Resolve the data directory for a given skill.
/// In dev: `<project>/skills/skills/<skill_id>/data/`
/// In production: `~/.alphahuman/skills/<skill_id>/data/`
fn resolve_data_dir(skill_id: &str) -> Result<PathBuf, String> {
    // Validate skill_id to prevent directory traversal
    if skill_id.contains("..") || skill_id.contains('/') || skill_id.contains('\\') {
        return Err("Invalid skill ID".to_string());
    }

    let current = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?;

    // Check: cwd/skills/skills/<id>/data (running from project root)
    let dev_data = current.join("skills").join("skills").join(skill_id).join("data");
    if current.join("skills").join("skills").exists() {
        return Ok(dev_data);
    }

    // Check: ../skills/skills/<id>/data (running from src-tauri/ via `tauri dev`)
    if let Some(parent) = current.parent() {
        let parent_data = parent.join("skills").join("skills").join(skill_id).join("data");
        if parent.join("skills").join("skills").exists() {
            return Ok(parent_data);
        }
    }

    // Production fallback
    let data_dir = crate::ai::encryption::get_data_dir()
        .unwrap_or_else(|_| PathBuf::from("data"));
    Ok(data_dir.join("skills").join(skill_id).join("data"))
}

/// Read a file from a skill's data directory.
#[tauri::command]
pub async fn skill_read_data(skill_id: String, filename: String) -> Result<String, String> {
    // Validate filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let data_dir = resolve_data_dir(&skill_id)?;
    let file_path = data_dir.join(&filename);

    tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read {}: {}", filename, e))
}

/// Write a file to a skill's data directory.
#[tauri::command]
pub async fn skill_write_data(
    skill_id: String,
    filename: String,
    content: String,
) -> Result<(), String> {
    // Validate filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }

    let data_dir = resolve_data_dir(&skill_id)?;

    // Ensure data directory exists
    tokio::fs::create_dir_all(&data_dir)
        .await
        .map_err(|e| format!("Failed to create data dir: {}", e))?;

    let file_path = data_dir.join(&filename);

    tokio::fs::write(&file_path, content.as_bytes())
        .await
        .map_err(|e| format!("Failed to write {}: {}", filename, e))
}

/// Get the resolved data directory path for a skill.
#[tauri::command]
pub async fn skill_data_dir(skill_id: String) -> Result<String, String> {
    let data_dir = resolve_data_dir(&skill_id)?;
    data_dir
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid path".to_string())
}

/// Resolve the venv site-packages path by scanning .venv/lib/ for a python3.* directory.
#[tauri::command]
pub async fn skill_venv_site_packages() -> Result<String, String> {
    // Reuse skill_cwd logic to find the skills directory
    let skills_dir_str = skill_cwd().await?;
    let skills_dir = PathBuf::from(skills_dir_str);

    let venv_lib = skills_dir.join(".venv").join("lib");
    if !venv_lib.exists() {
        return Err("No .venv/lib/ directory found".to_string());
    }

    // Scan for python3.* directories
    let entries = std::fs::read_dir(&venv_lib)
        .map_err(|e| format!("Failed to read .venv/lib/: {}", e))?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("python3") && entry.path().is_dir() {
            let site_packages = entry.path().join("site-packages");
            if site_packages.exists() {
                return site_packages
                    .to_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "Invalid path".to_string());
            }
        }
    }

    Err("No python3.*/site-packages found in .venv/lib/".to_string())
}

/// List manifest.json files from the skills directory.
#[tauri::command]
pub async fn skill_list_manifests() -> Result<Vec<serde_json::Value>, String> {
    let skills_cwd = skill_cwd().await?;
    let skills_dir = PathBuf::from(skills_cwd).join("skills");

    let mut manifests = Vec::new();

    let mut entries = tokio::fs::read_dir(&skills_dir)
        .await
        .map_err(|e| format!("Failed to read skills dir: {}", e))?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("manifest.json");
            if manifest_path.exists() {
                match tokio::fs::read_to_string(&manifest_path).await {
                    Ok(content) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                            manifests.push(parsed);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read manifest at {:?}: {}", manifest_path, e);
                    }
                }
            }
        }
    }

    Ok(manifests)
}
