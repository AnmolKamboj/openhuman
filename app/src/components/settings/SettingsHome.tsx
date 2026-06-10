import { type ReactNode, useState } from 'react';

import { useDeveloperMode } from '../../hooks/useDeveloperMode';
import { useT } from '../../lib/i18n/I18nContext';
import LanguageSelect from '../LanguageSelect';
import SettingsHeader from './components/SettingsHeader';
import SettingsMenuItem from './components/SettingsMenuItem';
import { useSettingsNavigation } from './hooks/useSettingsNavigation';
import SettingsSearchBar from './search/SettingsSearchBar';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SettingsItem {
  id: string;
  title: string;
  description: string;
  icon: ReactNode;
  onClick?: () => void;
  dangerous?: boolean;
  rightElement?: ReactNode;
}

interface SettingsGroup {
  /** Stable identifier for testing and key prop */
  id: string;
  /** i18n label shown above the card */
  label: string;
  items: SettingsItem[];
}

// ---------------------------------------------------------------------------
// Icon helpers (inline SVG kept as constants to avoid duplication)
// ---------------------------------------------------------------------------

const AccountIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
    />
  </svg>
);

const LanguageIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
    />
  </svg>
);

const AppearanceIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"
    />
  </svg>
);

const DevicesIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z"
    />
  </svg>
);

const PersonalityIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
    />
  </svg>
);

const MascotIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M12 21a9 9 0 100-18 9 9 0 000 18zM9 10h.01M15 10h.01M9.5 15c.83.67 1.67 1 2.5 1s1.67-.33 2.5-1"
    />
  </svg>
);

const PrivacyIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
    />
  </svg>
);

const NotificationsIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
    />
  </svg>
);

const DeveloperIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
    />
  </svg>
);

const AboutIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
    />
  </svg>
);

const DataSyncIcon = (
  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
    />
  </svg>
);

// ---------------------------------------------------------------------------
// Group header (visual separator label above each settings card)
// ---------------------------------------------------------------------------

const GroupHeader = ({ label }: { label: string }) =>
  label ? (
    <div className="px-1 pt-5 pb-1">
      <span className="text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400">
        {label}
      </span>
    </div>
  ) : (
    // Empty label → a plain divider (the doc places Developer & Diagnostics and
    // About after a divider, not under their own section headers).
    <div className="mx-1 mt-6 mb-2 border-t border-stone-200 dark:border-neutral-800" />
  );

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

const SettingsHome = () => {
  const { navigateToSettings } = useSettingsNavigation();
  const { t } = useT();
  const developerMode = useDeveloperMode();

  // Global settings search. While a query is active the normal menu is hidden
  // and the search bar renders its own ranked result list instead.
  const [searchQuery, setSearchQuery] = useState('');
  const isSearching = searchQuery.trim().length > 0;

  // --- 👤 Account group ---
  const accountGroup: SettingsGroup = {
    id: 'account',
    label: t('settings.groups.account'),
    items: [
      {
        // The Account row opens the account hub (recovery phrase, team,
        // connections, privacy, sign-out) — named after what it actually holds.
        id: 'profile',
        title: t('pages.settings.accountSection.title'),
        description: t('pages.settings.accountSection.description'),
        icon: AccountIcon,
        onClick: () => navigateToSettings('account'),
      },
      {
        id: 'language',
        title: t('settings.language'),
        description: t('settings.languageDesc'),
        icon: LanguageIcon,
        rightElement: <LanguageSelect ariaLabel={t('settings.language')} />,
      },
      {
        id: 'appearance',
        title: t('settings.appearance.title'),
        description: t('settings.appearance.menuDesc'),
        icon: AppearanceIcon,
        onClick: () => navigateToSettings('appearance'),
      },
      {
        id: 'devices',
        title: t('settings.account.devices'),
        description: t('settings.account.devicesDesc'),
        icon: DevicesIcon,
        onClick: () => navigateToSettings('devices'),
      },
      {
        id: 'data-sync',
        title: t('settings.dataSync.title'),
        description: t('settings.dataSync.menuDesc'),
        icon: DataSyncIcon,
        onClick: () => navigateToSettings('memory-sync'),
      },
    ],
  };

  // --- 🤖 Assistant group ---
  const assistantGroup: SettingsGroup = {
    id: 'assistant',
    label: t('settings.groups.assistant'),
    items: [
      {
        id: 'persona',
        title: t('settings.assistant.personality'),
        description: t('settings.assistant.personalityDesc'),
        icon: PersonalityIcon,
        onClick: () => navigateToSettings('persona'),
      },
      {
        id: 'mascot',
        title: t('settings.assistant.faceMascot'),
        description: t('settings.assistant.faceMascotDesc'),
        icon: MascotIcon,
        onClick: () => navigateToSettings('mascot'),
      },
    ],
  };

  // --- 🔒 Privacy group (Security + Approvals moved to Developer & Diagnostics) ---
  const privacySecurityGroup: SettingsGroup = {
    id: 'privacy-security',
    label: t('settings.privacySecurity.privacy'),
    items: [
      {
        id: 'privacy',
        title: t('settings.privacySecurity.privacy'),
        description: t('settings.privacySecurity.privacyDesc'),
        icon: PrivacyIcon,
        onClick: () => navigateToSettings('privacy'),
      },
    ],
  };

  // --- 🔔 Notifications group ---
  const notificationsGroup: SettingsGroup = {
    id: 'notifications',
    label: t('settings.groups.notifications'),
    items: [
      {
        id: 'notifications-hub',
        title: t('settings.notifications.menuTitle'),
        description: t('settings.notifications.menuDesc'),
        icon: NotificationsIcon,
        onClick: () => navigateToSettings('notifications-hub'),
      },
    ],
  };

  // --- ℹ️ About group (always visible; no section header — just a divider) ---
  const aboutGroup: SettingsGroup = {
    id: 'about',
    label: '',
    items: [
      {
        id: 'about',
        title: t('settings.about'),
        description: t('settings.aboutDesc'),
        icon: AboutIcon,
        onClick: () => navigateToSettings('about'),
      },
    ],
  };

  // --- Always-visible groups ---
  const visibleGroups: SettingsGroup[] = [
    accountGroup,
    assistantGroup,
    privacySecurityGroup,
    notificationsGroup,
  ];

  // Billing / Rewards / Wallet are NOT in Settings — per the design doc they
  // live in the avatar menu (monetisation out of the settings tree).

  // --- Developer & Diagnostics (gated) ---
  // The Developer & Diagnostics entry is hidden when developer mode is off.
  // About is always accessible — that's where the toggle lives (chicken-and-egg).
  // No section header — it sits after a divider, then About (per the doc).
  const developerGroup: SettingsGroup | null = developerMode
    ? {
        id: 'developer',
        label: '',
        items: [
          {
            id: 'developer-options',
            title: t('settings.developerDiagnostics'),
            description: t('settings.developerDiagnosticsDesc'),
            icon: DeveloperIcon,
            onClick: () => navigateToSettings('developer-options'),
          },
        ],
      }
    : null;

  // The layman groups (Account / Assistant / Privacy / Notifications) render as a
  // single flat card with no section subheadings. Developer & Diagnostics (when
  // on) and About sit after a divider, each in their own card.
  const laymanItems: SettingsItem[] = visibleGroups.flatMap(group => group.items);
  const trailingGroups: SettingsGroup[] = [...(developerGroup ? [developerGroup] : []), aboutGroup];

  return (
    <div className="z-10 relative">
      <div data-walkthrough="settings-menu">
        <SettingsHeader />
      </div>

      <SettingsSearchBar value={searchQuery} onValueChange={setSearchQuery} />

      {/* While searching, the search bar renders its own results and the normal
          settings menu is hidden to avoid a confusing double list. */}
      {isSearching ? null : (
        <div className="px-4 pt-3 pb-5">
          {/* Merged layman card — no Account/Assistant/… subheadings. */}
          <div
            data-testid="settings-group-main"
            className="rounded-3xl overflow-hidden border border-stone-200 dark:border-neutral-800">
            {laymanItems.map((item, index) => (
              <SettingsMenuItem
                key={item.id}
                icon={item.icon}
                title={item.title}
                description={item.description}
                onClick={item.onClick}
                testId={`settings-nav-${item.id}`}
                dangerous={item.dangerous}
                isFirst={index === 0}
                isLast={index === laymanItems.length - 1}
                rightElement={item.rightElement}
              />
            ))}
          </div>

          {trailingGroups.map(group => (
            <div key={group.id} data-testid={`settings-group-${group.id}`}>
              <GroupHeader label={group.label} />
              <div className="rounded-3xl overflow-hidden border border-stone-200 dark:border-neutral-800">
                {group.items.map((item, index) => (
                  <SettingsMenuItem
                    key={item.id}
                    icon={item.icon}
                    title={item.title}
                    description={item.description}
                    onClick={item.onClick}
                    testId={`settings-nav-${item.id}`}
                    dangerous={item.dangerous}
                    isFirst={index === 0}
                    isLast={index === group.items.length - 1}
                    rightElement={item.rightElement}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SettingsHome;
