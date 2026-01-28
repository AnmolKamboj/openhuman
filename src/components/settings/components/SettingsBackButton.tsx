interface SettingsBackButtonProps {
  onClick: () => void;
  title?: string;
  className?: string;
}

const SettingsBackButton = ({
  onClick,
  title = 'Settings',
  className = ''
}: SettingsBackButtonProps) => {
  return (
    <div className={`flex items-center p-4 border-b border-gray-100 ${className}`}>
      <button
        onClick={onClick}
        className="flex items-center space-x-3 text-gray-600 hover:text-gray-900 transition-colors duration-150"
        aria-label="Go back"
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
            d="M15 19l-7-7 7-7"
          />
        </svg>
        <span className="text-[15px] font-medium">{title}</span>
      </button>
    </div>
  );
};

export default SettingsBackButton;