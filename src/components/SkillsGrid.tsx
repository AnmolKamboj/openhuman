import { useState, useEffect } from "react";
import { useSkillConnectionStatus } from "../lib/skills/hooks";
import type { SkillConnectionStatus } from "../lib/skills/types";
import SkillSetupModal from "./skills/SkillSetupModal";

import TelegramIcon from "../assets/icons/telegram.svg";
import GoogleIcon from "../assets/icons/GoogleIcon";
import NotionIcon from "../assets/icons/notion.svg";

// Map skill IDs to icons
const SKILL_ICONS: Record<string, React.ReactElement> = {
  telegram: <img src={TelegramIcon} alt="Telegram" className="w-8 h-8" />,
  email: <GoogleIcon className="w-8 h-8" />,
  notion: <img src={NotionIcon} alt="Notion" className="w-8 h-8" />,
  github: (
    <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 24 24">
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  ),
  otter: (
    <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 24 24">
      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" />
    </svg>
  ),
};

// Default icon for unknown skills
const DefaultIcon = () => (
  <div className="w-8 h-8 rounded-full bg-primary-500/20 flex items-center justify-center">
    <svg className="w-4 h-4 text-primary-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
    </svg>
  </div>
);

// Status badge colors with better contrast
const STATUS_COLORS: Record<SkillConnectionStatus, { bg: string; text: string; border: string }> = {
  connected: {
    bg: "bg-sage-500/30",
    text: "text-sage-100",
    border: "border-sage-500/50",
  },
  connecting: {
    bg: "bg-amber-500/30",
    text: "text-amber-100",
    border: "border-amber-500/50",
  },
  not_authenticated: {
    bg: "bg-amber-500/30",
    text: "text-amber-100",
    border: "border-amber-500/50",
  },
  disconnected: {
    bg: "bg-stone-600/40",
    text: "text-stone-200",
    border: "border-stone-500/60",
  },
  error: {
    bg: "bg-coral-500/30",
    text: "text-coral-100",
    border: "border-coral-500/50",
  },
  offline: {
    bg: "bg-stone-600/40",
    text: "text-stone-200",
    border: "border-stone-500/60",
  },
  setup_required: {
    bg: "bg-primary-500/30",
    text: "text-primary-100",
    border: "border-primary-500/50",
  },
};

interface SkillCardProps {
  skillId: string;
  name: string;
  description: string;
  icon?: React.ReactElement;
  onConnect: () => void;
}

function SkillCard({ skillId, name, description: _description, icon, onConnect }: SkillCardProps) {
  const connectionStatus = useSkillConnectionStatus(skillId);
  const statusConfig = STATUS_COLORS[connectionStatus] || STATUS_COLORS.offline;

  return (
    <button
      onClick={onConnect}
      className="glass rounded-lg p-2.5 hover:bg-stone-800/30 transition-all duration-200 text-left focus:outline-none focus:ring-2 focus:ring-primary-500/50 group"
    >
      <div className="flex flex-col items-center space-y-1.5">
        {/* Icon */}
        <div className="w-8 h-8 flex items-center justify-center text-white opacity-80 group-hover:opacity-100 transition-opacity">
          {icon || <DefaultIcon />}
        </div>

        {/* Name */}
        <div className="text-center w-full">
          <div className="font-medium text-xs text-white">{name}</div>
        </div>

        {/* Status badge */}
        <div className={`px-1.5 py-0.5 text-[10px] font-medium rounded-full border ${statusConfig.bg} ${statusConfig.text} ${statusConfig.border}`}>
          {connectionStatus === "connected" && "Connected"}
          {connectionStatus === "connecting" && "Connecting..."}
          {connectionStatus === "not_authenticated" && "Not Auth"}
          {connectionStatus === "disconnected" && "Disconnected"}
          {connectionStatus === "error" && "Error"}
          {connectionStatus === "offline" && "Offline"}
          {connectionStatus === "setup_required" && "Setup"}
        </div>
      </div>
    </button>
  );
}

interface SkillCatalogEntry {
  name: string;
  description: string;
  icon: string | null;
  version: string;
  tools: string[];
  hooks: string[];
  tickIntervalMinutes: number | null;
  path: string;
}

interface SkillsCatalog {
  generatedAt: string;
  version: string;
  skills: SkillCatalogEntry[];
}

export default function SkillsGrid() {
  const [skillsList, setSkillsList] = useState<Array<{
    id: string;
    name: string;
    description: string;
    icon?: React.ReactElement;
  }>>([]);
  const [loading, setLoading] = useState(true);
  const [setupModalOpen, setSetupModalOpen] = useState(false);
  const [activeSkillId, setActiveSkillId] = useState<string | null>(null);
  const [activeSkillName, setActiveSkillName] = useState<string>("");

  useEffect(() => {
    // Load skills catalog from the skills repo
    // The file should be copied to public/skills-catalog.json during build
    const loadSkillsCatalog = async () => {
      try {
        // Try to load from public folder (served at root in Vite)
        const response = await fetch("/skills-catalog.json");
        if (!response.ok) {
          // Fallback: try skills submodule path
          const fallbackResponse = await fetch("/skills/skills-catalog.json");
          if (!fallbackResponse.ok) {
            console.warn("Could not load skills-catalog.json. Make sure it's copied to public/ folder.");
            setLoading(false);
            return;
          }
          const catalog: SkillsCatalog = await fallbackResponse.json();
          processCatalog(catalog);
          return;
        }
        const catalog: SkillsCatalog = await response.json();
        processCatalog(catalog);
      } catch (error) {
        console.error("Error loading skills catalog:", error);
        setLoading(false);
      }
    };

    const processCatalog = (catalog: SkillsCatalog) => {
      // Filter skills that have setup hooks and validate skill names
      const skillsWithSetup = catalog.skills.filter((skill) => {
        // Skip skills with underscores in name (used for tool namespacing)
        if (skill.name.includes("_")) {
          console.warn(
            `Skill "${skill.name}" contains underscore and will be skipped. Skill names cannot contain underscores.`
          );
          return false;
        }
        return (
          skill.hooks.includes("on_setup_start") &&
          skill.hooks.includes("on_setup_submit") &&
          skill.hooks.includes("on_setup_cancel")
        );
      });

      const processed = skillsWithSetup.map((skill) => ({
        id: skill.name,
        name: skill.name.charAt(0).toUpperCase() + skill.name.slice(1),
        description: skill.description,
        icon: SKILL_ICONS[skill.name],
      }));

      setSkillsList(processed);
      setLoading(false);
    };

    loadSkillsCatalog();
  }, []);

  // If loading or no skills, don't render
  if (loading || skillsList.length === 0) {
    return null;
  }

  const handleConnect = (skillId: string, skillName: string) => {
    setActiveSkillId(skillId);
    setActiveSkillName(skillName);
    setSetupModalOpen(true);
  };

  return (
    <>
      <div className="animate-fade-up mt-4 mb-8">
        <h3 className="text-sm font-semibold text-white mb-4 px-1 text-center opacity-80">Available Skills</h3>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          {skillsList.map((skill) => (
            <SkillCard
              key={skill.id}
              skillId={skill.id}
              name={skill.name}
              description={skill.description}
              icon={skill.icon}
              onConnect={() => handleConnect(skill.id, skill.name)}
            />
          ))}
        </div>
      </div>

      {/* Setup modal */}
      {setupModalOpen && activeSkillId && (
        <SkillSetupModal
          skillId={activeSkillId}
          skillName={activeSkillName}
          onClose={() => {
            setSetupModalOpen(false);
            setActiveSkillId(null);
          }}
        />
      )}
    </>
  );
}
