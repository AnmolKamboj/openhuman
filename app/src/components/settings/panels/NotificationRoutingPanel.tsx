import { useEffect, useState } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import {
  fetchNotificationStats,
  getNotificationSettings,
  setNotificationSettings,
} from '../../../services/notificationService';
import type { NotificationStats } from '../../../types/notifications';
import Alert, { AlertDescription, AlertTitle } from '../../ui/Alert';
import Badge, { type BadgeVariant } from '../../ui/Badge';
import Slider from '../../ui/Slider';
import { SettingsCheckbox, SettingsSection } from '../controls';
import SettingsPanel from '../layout/SettingsPanel';

const PROVIDERS = ['gmail', 'slack', 'discord', 'whatsapp'];

interface NotificationRoutingPanelProps {
  /** When embedded inside the tabbed Notifications page, the parent owns the
      `<SettingsHeader>` chrome and we render only the body. */
  embedded?: boolean;
}

/**
 * Settings panel for the notification intelligence / routing pipeline.
 *
 * Currently exposes a global explanation card. Per-provider threshold
 * controls will populate here as providers are connected.
 */
const NotificationRoutingPanel = ({ embedded = false }: NotificationRoutingPanelProps = {}) => {
  const { t } = useT();
  const providers = PROVIDERS;
  const [stats, setStats] = useState<NotificationStats | null>(null);
  const [settings, setSettings] = useState<
    Record<
      string,
      { enabled: boolean; importance_threshold: number; route_to_orchestrator: boolean }
    >
  >({});
  const [loadedProviders, setLoadedProviders] = useState<Record<string, boolean>>({});
  const [loadErrors, setLoadErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    void fetchNotificationStats()
      .then(s => setStats(s))
      .catch(err => console.warn('[settings][notification-routing] stats load failed', err));
  }, []);

  useEffect(() => {
    void Promise.allSettled(
      providers.map(async provider => {
        const s = await getNotificationSettings(provider);
        return [provider, s] as const;
      })
    ).then(results => {
      const next: Record<
        string,
        { enabled: boolean; importance_threshold: number; route_to_orchestrator: boolean }
      > = {};
      const nextLoadedProviders: Record<string, boolean> = {};
      const nextLoadErrors: Record<string, string> = {};
      results.forEach((result, index) => {
        const provider = providers[index];
        if (result.status === 'fulfilled') {
          const [, s] = result.value;
          next[provider] = {
            enabled: s.enabled,
            importance_threshold: s.importance_threshold,
            route_to_orchestrator: s.route_to_orchestrator,
          };
          nextLoadedProviders[provider] = true;
        } else {
          const message =
            result.reason instanceof Error ? result.reason.message : String(result.reason);
          nextLoadErrors[provider] = message;
          console.warn(`[settings][notification-routing] failed to load provider=${provider}`, {
            error: message,
          });
        }
      });
      setSettings(prev => ({ ...prev, ...next }));
      setLoadedProviders(nextLoadedProviders);
      setLoadErrors(nextLoadErrors);
    });
  }, [providers]);

  const updateSetting = async (
    provider: string,
    patch: Partial<{
      enabled: boolean;
      importance_threshold: number;
      route_to_orchestrator: boolean;
    }>
  ) => {
    if (!loadedProviders[provider] || loadErrors[provider]) {
      return;
    }
    const current = settings[provider] ?? {
      enabled: true,
      importance_threshold: 0,
      route_to_orchestrator: true,
    };
    const next = { ...current, ...patch };
    setSettings(prev => ({ ...prev, [provider]: next }));
    try {
      await setNotificationSettings({ provider, ...next });
    } catch (err) {
      setSettings(prev => ({ ...prev, [provider]: current }));
      throw err;
    }
  };

  const body = (
    <>
      {stats && (
        <SettingsSection title={t('notifications.routing.pipelineStats')}>
          <div className="grid grid-cols-3 divide-x divide-line-subtle">
            {[
              { label: t('notifications.routing.total'), value: stats.total },
              { label: t('notifications.routing.unread'), value: stats.unread },
              { label: t('notifications.routing.unscored'), value: stats.unscored },
            ].map(({ label, value }) => (
              <div key={label} className="px-4 py-3 text-center">
                <p className="text-lg font-semibold text-content">{value}</p>
                <p className="text-xs text-content-muted">{label}</p>
              </div>
            ))}
          </div>
        </SettingsSection>
      )}

      {/* Info card */}
      <Alert variant="info" className="items-start">
        <svg
          className="w-5 h-5 shrink-0 mt-0.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
          />
        </svg>
        <div>
          <AlertTitle>{t('notifications.routing.intelligenceTitle')}</AlertTitle>
          <AlertDescription className="mt-1">
            {t('notifications.routing.intelligenceDesc')}
          </AlertDescription>
        </div>
      </Alert>

      {/* How it works */}
      <SettingsSection title={t('notifications.routing.howItWorks')}>
        {(
          [
            {
              label: t('notifications.routing.level.drop'),
              desc: t('notifications.routing.level.dropDesc'),
              variant: 'neutral',
            },
            {
              label: t('notifications.routing.level.acknowledge'),
              desc: t('notifications.routing.level.acknowledgeDesc'),
              variant: 'primary',
            },
            {
              label: t('notifications.routing.level.react'),
              desc: t('notifications.routing.level.reactDesc'),
              variant: 'warning',
            },
            {
              label: t('notifications.routing.level.escalate'),
              desc: t('notifications.routing.level.escalateDesc'),
              variant: 'danger',
            },
          ] as { label: string; desc: string; variant: BadgeVariant }[]
        ).map(row => (
          <div key={row.label} className="flex items-center gap-3 px-4 py-3">
            <Badge variant={row.variant} className="shrink-0">
              {row.label}
            </Badge>
            <span className="text-xs text-content-secondary">{row.desc}</span>
          </div>
        ))}
      </SettingsSection>

      {/* Per-provider routing */}
      <SettingsSection title={t('notifications.routing.perProvider')}>
        {providers.map(provider => {
          const hasLoadError = Boolean(loadErrors[provider]);
          const isLoaded = Boolean(loadedProviders[provider]);
          const s = settings[provider] ?? {
            enabled: true,
            importance_threshold: 0,
            route_to_orchestrator: true,
          };
          const controlsDisabled = !isLoaded || hasLoadError;
          return (
            <div key={provider} className="px-4 py-3 space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-sm font-medium text-content capitalize">{provider}</p>
                <div className="flex items-center gap-2">
                  <label
                    htmlFor={`notification-enabled-${provider}`}
                    className="text-xs text-content-secondary">
                    {t('common.enabled')}
                  </label>
                  <SettingsCheckbox
                    id={`notification-enabled-${provider}`}
                    checked={s.enabled}
                    disabled={controlsDisabled}
                    onCheckedChange={next => {
                      void updateSetting(provider, { enabled: next });
                    }}
                  />
                </div>
              </div>
              <div className="flex items-center gap-2 text-xs text-content-secondary">
                <span className="shrink-0">{t('notifications.routing.threshold')}</span>
                <Slider
                  className="flex-1"
                  min={0}
                  max={1}
                  step={0.05}
                  value={[s.importance_threshold]}
                  disabled={controlsDisabled}
                  thumbLabels={[t('notifications.routing.threshold')]}
                  data-testid={`notification-threshold-${provider}`}
                  onValueChange={value => {
                    void updateSetting(provider, { importance_threshold: value[0] });
                  }}
                />
                <span>{s.importance_threshold.toFixed(2)}</span>
              </div>
              <label
                htmlFor={`notification-orchestrator-${provider}`}
                className="text-xs text-content-secondary flex items-center gap-2">
                {t('notifications.routing.routeToOrchestrator')}
                <SettingsCheckbox
                  id={`notification-orchestrator-${provider}`}
                  checked={s.route_to_orchestrator}
                  disabled={controlsDisabled}
                  onCheckedChange={next => {
                    void updateSetting(provider, { route_to_orchestrator: next });
                  }}
                />
              </label>
              {hasLoadError ? (
                <p className="text-xs text-coral-600 dark:text-coral-300">
                  {t('notifications.routing.loadSettingsError')}
                </p>
              ) : null}
            </div>
          );
        })}
      </SettingsSection>
    </>
  );

  // Embedded inside the tabbed Notifications page: the parent owns the header,
  // so render just the padded body.
  if (embedded) return <div className="p-4 space-y-5">{body}</div>;

  return <SettingsPanel>{body}</SettingsPanel>;
};

export default NotificationRoutingPanel;
