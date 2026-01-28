import { ReactNode } from 'react';

interface SettingsMenuItemProps {
  icon: ReactNode;
  title: string;
  description?: string;
  onClick: () => void;
  dangerous?: boolean;
  className?: string;
}

const SettingsMenuItem = ({
  icon,
  title,
  description,
  onClick,
  dangerous = false,
  className = ''
}: SettingsMenuItemProps) => {
  const textColor = dangerous ? 'text-red-600' : 'text-gray-900';
  const descriptionColor = dangerous ? 'text-red-500/70' : 'text-gray-500';

  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center justify-between h-[52px] px-6 hover:bg-gray-50 active:bg-gray-100 transition-colors duration-150 text-left ${className}`}
    >
      <div className="flex items-center space-x-3 flex-1 min-w-0">
        <div className={`flex-shrink-0 ${dangerous ? 'text-red-500' : 'text-gray-600'}`}>
          {icon}
        </div>
        <div className="flex-1 min-w-0">
          <p className={`text-[15px] font-medium ${textColor} truncate`}>
            {title}
          </p>
          {description && (
            <p className={`text-sm ${descriptionColor} truncate`}>
              {description}
            </p>
          )}
        </div>
      </div>

      {/* Chevron */}
      <div className="flex-shrink-0 ml-3 transition-transform duration-150 hover:translate-x-0.5">
        <svg
          className="w-5 h-5 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 5l7 7-7 7"
          />
        </svg>
      </div>
    </button>
  );
};

export default SettingsMenuItem;