import { useAppSelector } from '../../../store/hooks';

interface SettingsHeaderProps {
  className?: string;
}

const SettingsHeader = ({ className = '' }: SettingsHeaderProps) => {
  const user = useAppSelector((state) => state.user.user);

  // Get user initials for avatar
  const getInitials = (firstName?: string, lastName?: string): string => {
    if (!firstName && !lastName) return 'U';
    const first = firstName?.charAt(0) || '';
    const last = lastName?.charAt(0) || '';
    return (first + last).toUpperCase();
  };

  const initials = getInitials(user?.firstName, user?.lastName);
  const displayName = user?.firstName && user?.lastName
    ? `${user.firstName} ${user.lastName}`
    : user?.firstName || user?.username || 'User';
  const displayUsername = user?.username ? `@${user.username}` : '';

  return (
    <div className={`bg-black/30 border-b border-stone-700 p-6 ${className}`}>
      <div className="flex items-center space-x-4">
        {/* Avatar */}
        <div className="w-14 h-14 bg-stone-700 rounded-full flex items-center justify-center flex-shrink-0">
          <span className="text-white font-semibold text-lg">{initials}</span>
        </div>

        {/* User info */}
        <div className="flex-1 min-w-0">
          <h2 className="text-lg font-semibold text-white truncate" id="settings-modal-title">
            {displayName}
          </h2>
          {displayUsername && (
            <p className="text-sm text-stone-400 truncate">{displayUsername}</p>
          )}
        </div>
      </div>
    </div>
  );
};

export default SettingsHeader;