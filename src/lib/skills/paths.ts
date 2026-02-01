/**
 * Path resolution for skills directories.
 *
 * Dev mode: skills are in the git submodule at `skills/skills/`
 * Production: skills are in `~/.alphahuman/skills/`
 */

import { IS_DEV } from "../../utils/config";

/**
 * Get the root directory for discovering skills.
 * In dev, this is the submodule `skills/` dir (which contains `skills/<name>/`).
 * In production, this is the user's app data directory.
 */
export function getSkillsBaseDir(): string {
  if (IS_DEV) {
    // In dev, the Tauri app's cwd is the project root.
    // The submodule is at ./skills/ and skills live at ./skills/skills/<name>/
    return "skills";
  }
  // Production: resolved by Rust command (appDataDir based)
  return "";
}

/**
 * Get the working directory for spawning a skill subprocess.
 * The Python package structure expects `python -m skills.telegram`
 * to be run from the repo root (the `skills/` submodule directory).
 */
export function getSkillCwd(): string {
  if (IS_DEV) {
    return "skills";
  }
  return "";
}

/**
 * Get the module path for a skill given its ID.
 * e.g. "telegram" → ["skills.telegram"] for `python -m skills.telegram`
 */
export function getSkillModulePath(skillId: string): string {
  return `skills.${skillId}`;
}

/**
 * Get the data directory path for a skill.
 * Data is persisted per-skill in an isolated directory.
 */
export function getSkillDataDir(skillId: string): string {
  if (IS_DEV) {
    return `skills/skills/${skillId}/data`;
  }
  return `skills/${skillId}/data`;
}

/**
 * Get the manifest path for a skill.
 */
export function getSkillManifestPath(skillId: string): string {
  if (IS_DEV) {
    return `skills/skills/${skillId}/manifest.json`;
  }
  return `skills/${skillId}/manifest.json`;
}
