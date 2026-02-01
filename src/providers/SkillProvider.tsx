/**
 * Skill Provider — discovers and manages skill lifecycles.
 *
 * On mount (when authenticated): discovers skills from the submodule,
 * registers them in Redux, and auto-starts skills with completed setup.
 * On unmount: stops all running skills.
 */

import { useEffect, useRef, type ReactNode } from "react";
import { useAppSelector } from "../store/hooks";
import { skillManager } from "../lib/skills/manager";
import type { SkillManifest } from "../lib/skills/types";

// Hard-coded manifest for the telegram skill (discovered from submodule).
// In production, this would be dynamically read from manifest.json files.
const KNOWN_SKILLS: SkillManifest[] = [
  {
    id: "telegram",
    name: "Telegram",
    version: "2.0.0",
    description:
      "Telegram integration via Telethon MTProto — 75+ tools for chats, messages, contacts, admin, media, and settings.",
    runtime: "python",
    entry: "__main__.py",
    tick_interval: 1_200_000,
    env: ["TELEGRAM_API_ID", "TELEGRAM_API_HASH"],
    dependencies: [
      "telethon>=1.36.0",
      "mcp>=1.0.0",
      "pydantic>=2.0",
      "aiosqlite>=0.20.0",
    ],
    setup: {
      required: true,
      label: "Connect Telegram",
    },
  },
];

export default function SkillProvider({ children }: { children: ReactNode }) {
  const { token } = useAppSelector((state) => state.auth);
  const skillsState = useAppSelector((state) => state.skills.skills);
  const initRef = useRef(false);

  useEffect(() => {
    if (!token) return;
    if (initRef.current) return;
    initRef.current = true;

    // Register known skills
    for (const manifest of KNOWN_SKILLS) {
      skillManager.registerSkill(manifest);
    }

    // Auto-start skills that have completed setup
    for (const manifest of KNOWN_SKILLS) {
      const existing = skillsState[manifest.id];
      if (existing?.setupComplete) {
        skillManager.startSkill(manifest).catch((err) => {
          console.error(`[SkillProvider] Failed to start ${manifest.id}:`, err);
        });
      }
    }

    return () => {
      // Cleanup on unmount
      skillManager.stopAll().catch(console.error);
      initRef.current = false;
    };
  }, [token]); // eslint-disable-line react-hooks/exhaustive-deps

  return <>{children}</>;
}
