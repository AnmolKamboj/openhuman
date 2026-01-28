import { ReactNode } from 'react';
import SettingsBackButton from './SettingsBackButton';

interface SettingsPanelLayoutProps {
  title: string;
  onBack: () => void;
  children: ReactNode;
  className?: string;
}

const SettingsPanelLayout = ({
  title,
  onBack,
  children,
  className = ''
}: SettingsPanelLayoutProps) => {
  return (
    <div className={`bg-white rounded-2xl overflow-hidden ${className}`}>
      <SettingsBackButton onClick={onBack} title={title} />
      <div className="max-h-[70vh] overflow-y-auto">
        {children}
      </div>
    </div>
  );
};

export default SettingsPanelLayout;