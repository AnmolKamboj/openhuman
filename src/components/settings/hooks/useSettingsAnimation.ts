import { useState, useEffect } from 'react';

export type AnimationState = 'entering' | 'entered' | 'exiting' | 'exited';

interface SettingsAnimationHook {
  isVisible: boolean;
  animationState: AnimationState;
  startEntry: () => void;
  startExit: () => void;
}

export const useSettingsAnimation = (
  duration = 200
): SettingsAnimationHook => {
  const [animationState, setAnimationState] = useState<AnimationState>('exited');

  const isVisible = animationState === 'entering' || animationState === 'entered';

  const startEntry = () => {
    setAnimationState('entering');
    setTimeout(() => {
      setAnimationState('entered');
    }, duration);
  };

  const startExit = () => {
    setAnimationState('exiting');
    setTimeout(() => {
      setAnimationState('exited');
    }, duration);
  };

  return {
    isVisible,
    animationState,
    startEntry,
    startExit
  };
};

// Hook for panel slide animations (slide from right)
export const usePanelAnimation = (isActive: boolean, duration = 250) => {
  const [mounted, setMounted] = useState(isActive);

  useEffect(() => {
    if (isActive) {
      setMounted(true);
    } else {
      const timer = setTimeout(() => setMounted(false), duration);
      return () => clearTimeout(timer);
    }
  }, [isActive, duration]);

  const getPanelClasses = () => {
    const baseClasses = 'transition-all duration-250 ease-out';
    if (!mounted) return `${baseClasses} opacity-0`;

    return isActive
      ? `${baseClasses} opacity-100 translate-x-0`
      : `${baseClasses} opacity-0 translate-x-full`;
  };

  return {
    mounted,
    panelClasses: getPanelClasses()
  };
};