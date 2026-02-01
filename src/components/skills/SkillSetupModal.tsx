/**
 * Modal wrapper for the skill setup wizard.
 * Uses createPortal like the settings modal system.
 */

import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import SkillSetupWizard from "./SkillSetupWizard";

interface SkillSetupModalProps {
  skillId: string;
  skillName: string;
  onClose: () => void;
}

export default function SkillSetupModal({
  skillId,
  skillName,
  onClose,
}: SkillSetupModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  // Handle escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [onClose]);

  // Focus management
  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement;
    if (modalRef.current) {
      modalRef.current.focus();
    }
    return () => {
      if (previousFocus?.focus) {
        previousFocus.focus();
      }
    };
  }, []);

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  const modalContent = (
    <div
      className="fixed inset-0 z-[9999] bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={handleBackdropClick}
      role="dialog"
      aria-modal="true"
      aria-labelledby="skill-setup-title"
    >
      <div
        ref={modalRef}
        className="glass rounded-3xl shadow-large w-full max-w-[460px] overflow-hidden animate-fade-up focus:outline-none focus:ring-0"
        style={{
          animationDuration: "200ms",
          animationTimingFunction: "cubic-bezier(0.25, 0.46, 0.45, 0.94)",
          animationFillMode: "both",
        }}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-stone-700/50">
          <h2
            id="skill-setup-title"
            className="text-base font-semibold text-white"
          >
            Connect {skillName}
          </h2>
          <button
            onClick={onClose}
            className="p-1 text-stone-400 hover:text-white transition-colors rounded-lg hover:bg-stone-700/50"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Wizard content */}
        <div className="p-4">
          <SkillSetupWizard
            skillId={skillId}
            onComplete={onClose}
            onCancel={onClose}
          />
        </div>
      </div>
    </div>
  );

  return createPortal(modalContent, document.body);
}
