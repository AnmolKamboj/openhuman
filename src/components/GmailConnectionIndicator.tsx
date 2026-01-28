interface GmailConnectionIndicatorProps {
  description?: string;
  className?: string;
}

const GmailConnectionIndicator = ({
  description,
  className = '',
}: GmailConnectionIndicatorProps) => {
  // Gmail is always offline for now (placeholder)
  const gmailIsOnline = false;

  return (
    <div className={`mb-6 ${className}`}>
      <div className="flex items-center justify-center space-x-2 mb-3">
        <div className={`w-2 h-2 ${gmailIsOnline ? 'bg-red-500' : 'bg-gray-500'} rounded-full ${gmailIsOnline ? 'animate-pulse' : ''}`}></div>
        <div className="flex items-center space-x-1.5">
          {/* Gmail icon */}
          <svg className="w-4 h-4 text-red-500" fill="currentColor" viewBox="0 0 24 24">
            <path d="M24 5.5v13.05c0 .85-.73 1.59-1.59 1.59H1.59C.73 21.14 0 20.4 0 19.55V5.5L12 13.25 24 5.5zM24 4.5c0-.42-.2-.83-.53-1.09L12 11.25.53 3.41C.2 3.67 0 4.08 0 4.5v.75L12 13 24 5.25V4.5z"/>
            <path d="M5.5 4.5L12 9.75 18.5 4.5H5.5z" opacity="0.3"/>
          </svg>
          <span className={`text-sm ${gmailIsOnline ? 'text-red-500' : 'text-gray-500'}`}>
            {gmailIsOnline ? 'Connected to Gmail' : 'Gmail is Offline'}
          </span>
        </div>
      </div>
      {description && (
        <p className="text-xs opacity-60 text-center leading-relaxed">
          {description}
        </p>
      )}
    </div>
  );
};

export default GmailConnectionIndicator;
