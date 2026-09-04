/**
 * Unified memory sources panel.
 *
 * Single source of truth for **what feeds memory**: folders, GitHub
 * repos, RSS feeds, web pages, Twitter queries, and Composio
 * integrations. Polls `openhuman.memory_sources_status_list` every 5s
 * for per-source chunk counts and freshness. The Sync button on each
 * row dispatches `openhuman.memory_sources_sync` which runs in the
 * background and emits MemorySyncStageChanged events.
 */
import { useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useT } from '../../lib/i18n/I18nContext';
import { CoreStateContext } from '../../providers/coreStateContext';
import {
  applyAllIn,
  listMemorySources,
  type MemorySourceEntry,
  memorySourcesStatusList,
  removeMemorySource,
  type SourceStatus,
  syncMemorySource,
  updateMemorySource,
} from '../../services/memorySourcesService';
import type {
  ConfirmationModal as ConfirmationModalType,
  ToastNotification,
} from '../../types/intelligence';
import {
  memoryTreeFlushSource,
  memoryTreePipelineStatus,
  type MemoryTreePipelineStatus,
} from '../../utils/tauriCommands/memoryTree';
import { Card } from '../ui';
import Button from '../ui/Button';
import { AddMemorySourceDialog } from './AddMemorySourceDialog';
import { ConfirmationModal } from './ConfirmationModal';
import { MemorySourceRow } from './MemorySourceRow';
import { AllInIcon, PlusIcon } from './memorySourcesIcons';
import { sourceTreeScope } from './memorySourcesRowHelpers';
import {
  noteSyncRejected,
  noteSyncRequested,
  reconcileWithStatuses,
  subscribeTerminalSyncEvents,
  useMemorySyncActivity,
} from './memorySyncActivityStore';
import { MemorySyncSchedule } from './MemorySyncSchedule';

// The parsers moved to the store with the state they feed (openhuman#6019);
// re-exported so their tests and any other reader keep one import path.
export { parseIngestedCount, parseSyncProgress } from './memorySyncActivityStore';

interface MemorySourcesRegistryProps {
  onToast?: (toast: Omit<ToastNotification, 'id'>) => void;
  pollIntervalMs?: number;
}

export function MemorySourcesRegistry({
  onToast,
  pollIntervalMs = 5000,
}: MemorySourcesRegistryProps) {
  const { t } = useT();
  const navigate = useNavigate();
  // Read the core snapshot directly (not via the throwing `useCoreState`
  // hook) so this component still renders in unit tests that mount it
  // without a CoreStateProvider — there `ctx` is null and `isAuthenticated`
  // stays a stable `false`, so the load effect behaves exactly as before.
  const coreState = useContext(CoreStateContext);
  const isAuthenticated = coreState?.snapshot.auth.isAuthenticated ?? false;
  const [sources, setSources] = useState<MemorySourceEntry[]>([]);
  const [statuses, setStatuses] = useState<SourceStatus[]>([]);
  // Global downstream pipeline health (GH-4690) — the same snapshot the
  // Brain > Memory > Sync panel renders. Folded into each row so a source that
  // ingested but failed embeddings/extraction/tree-build shows a warning rather
  // than a clean synced badge. `null` until the first poll resolves.
  const [pipeline, setPipeline] = useState<MemoryTreePipelineStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  // The live sync state — which rows are syncing, their bar, their last
  // result — lives in `memorySyncActivityStore`, which outlives this screen
  // (openhuman#6019): a tab or route change mid-sync used to drop it all and
  // miss the terminal event, so the row came back idle as if the sync had
  // stopped. The store's listener is installed once for the page; this screen
  // only reads.
  const { syncingIds, progress: syncProgress, results: syncResults } = useMemorySyncActivity();
  const [buildingId, setBuildingId] = useState<string | null>(null);
  const [allInModalOpen, setAllInModalOpen] = useState(false);
  const [applyingAllIn, setApplyingAllIn] = useState(false);
  const allInInFlightRef = useRef(false);
  const [expandedSettingsId, setExpandedSettingsId] = useState<string | null>(null);

  // Refs let the (intentionally dep-free) sync-stage listener fire accurate
  // toasts on the *terminal* event without re-subscribing on every render or
  // 5s poll. The handler must read the latest onToast/sources/t (#3295).
  const onToastRef = useRef(onToast);
  const sourcesRef = useRef(sources);
  const tRef = useRef(t);
  useEffect(() => {
    onToastRef.current = onToast;
    sourcesRef.current = sources;
    tRef.current = t;
  });

  // The toast for a run that ends while this screen is mounted. The state
  // itself is the store's; a run that ends while no screen is showing still
  // lands as the row's result chip on the next mount, it just goes untoasted.
  useEffect(
    () =>
      subscribeTerminalSyncEvents(({ rowId, stage, detail, result }) => {
        const tt = tRef.current;
        const label = sourcesRef.current.find(s => s.id === rowId)?.label ?? rowId;
        if (stage === 'completed') {
          // The item count parsed from the detail ("ingested N item(s)");
          // 0 new items → "up to date" (#3295).
          const items = result.items;
          onToastRef.current?.({
            type: 'success',
            title: `${tt('memorySources.sync.completeTitle')} ${label}`,
            message:
              items && items > 0
                ? `${items} ${tt('memorySources.sync.itemsSynced')}`
                : tt('memorySources.sync.upToDate'),
          });
        } else {
          // The core already reported internal bugs to Sentry via
          // report_error_or_expected; here the reason reaches the user.
          onToastRef.current?.({
            type: 'error',
            title: `${tt('memorySources.sync.failedLabel')} · ${label}`,
            message: detail ?? tt('memorySources.sync.failedLabel'),
          });
        }
      }),
    []
  );

  const refresh = useCallback(async () => {
    try {
      const [list, stats, health] = await Promise.all([
        listMemorySources().catch(err => {
          console.warn('[ui-flow][memory-sources] list failed', err);
          return [] as MemorySourceEntry[];
        }),
        memorySourcesStatusList().catch(err => {
          console.warn('[ui-flow][memory-sources] status_list failed', err);
          return [] as SourceStatus[];
        }),
        // GH-4690: downstream pipeline health. Best-effort — a failure here
        // must never hide the source list, so we fall back to `null` (rows then
        // rely on the precise per-source pending-chunk signal alone).
        memoryTreePipelineStatus().catch(err => {
          console.warn('[ui-flow][memory-sources] pipeline_status failed', err);
          return null;
        }),
      ]);
      setSources(list);
      setStatuses(stats);
      setPipeline(health);
      // The core's own account of what is in flight (`sync_stage`,
      // openhuman#6019): seeds the bar for a run this page never saw start —
      // a cold mount, an app reload mid-sync — and takes down a bar whose
      // terminal event was lost, once the core has reported the row idle for
      // longer than the grace. RC#5's "rehydrates via poll" promise (#3295)
      // finally has the field it needed.
      reconcileWithStatuses(stats);
    } finally {
      setLoading(false);
    }
  }, []);

  // Load on mount AND whenever the session transitions to authenticated.
  // After a page reload the registry can mount (e.g. via a persisted
  // `?tab=memory` deep link) *before* CoreStateProvider has restored the
  // session, so the initial fetch runs against a not-yet-ready core and
  // surfaces nothing. Re-running when `isAuthenticated` flips true picks up
  // sources immediately instead of waiting for the next 5s poll — which
  // under CI load was racing the E2E visibility timeout (#3449).
  useEffect(() => {
    void refresh();
  }, [refresh, isAuthenticated]);

  useEffect(() => {
    if (!pollIntervalMs) return undefined;
    const id = setInterval(() => {
      void refresh();
    }, pollIntervalMs);
    return () => clearInterval(id);
  }, [pollIntervalMs, refresh]);

  const statusById = useMemo(() => {
    const m = new Map<string, SourceStatus>();
    for (const s of statuses) m.set(s.source_id, s);
    return m;
  }, [statuses]);

  // Newest chunk timestamp across every source — the "Last synced …" anchor
  // for the global schedule header. Derived from persisted chunk data, so it
  // survives restarts.
  const overallLastSyncMs = useMemo(() => {
    let newest: number | null = null;
    for (const s of statuses) {
      if (s.last_chunk_at_ms != null && (newest === null || s.last_chunk_at_ms > newest)) {
        newest = s.last_chunk_at_ms;
      }
    }
    return newest;
  }, [statuses]);

  const handleToggle = useCallback(
    async (source: MemorySourceEntry) => {
      try {
        const updated = await updateMemorySource(source.id, { enabled: !source.enabled });
        setSources(prev => prev.map(s => (s.id === updated.id ? updated : s)));
      } catch (err) {
        onToast?.({
          type: 'error',
          title: t('memorySources.toggleFailed'),
          message: err instanceof Error ? err.message : String(err),
        });
      }
    },
    [onToast, t]
  );

  const handleRemove = useCallback(
    async (source: MemorySourceEntry) => {
      try {
        await removeMemorySource(source.id);
        setSources(prev => prev.filter(s => s.id !== source.id));
        onToast?.({ type: 'success', title: t('memorySources.removed'), message: source.label });
      } catch (err) {
        onToast?.({
          type: 'error',
          title: t('memorySources.removeFailed'),
          message: err instanceof Error ? err.message : String(err),
        });
      }
    },
    [onToast, t]
  );

  const handleSync = useCallback(
    async (source: MemorySourceEntry) => {
      // RC#1 (#3295): light the row on click — the event will also fire, but
      // this way it lights before the first sync-stage event arrives — and
      // drop the prior run's result chip.
      noteSyncRequested(source.id);
      console.debug(`[ui-flow][memory-sync] manual sync triggered source_id=${source.id}`);
      try {
        await syncMemorySource(source.id);
        // NOTE: success/failure feedback is intentionally NOT fired here. This
        // RPC returns in ~4ms after merely *spawning* the background sync; the
        // real outcome arrives via the terminal `completed`/`failed` stage event
        // (handled above), which carries the item count / failure reason (#3295).
        void refresh();
      } catch (err) {
        // The RPC call itself failed (transport/validation) — the background
        // sync never started, so no stage event will arrive. Surface it here:
        // clear the syncing flag and record a failed result on the row.
        const reason = err instanceof Error ? err.message : String(err);
        noteSyncRejected(source.id, reason);
        onToast?.({
          type: 'error',
          title: `${t('memorySources.sync.failedLabel')} · ${source.label}`,
          message: reason,
        });
      }
      // No `finally` clear: on success the row stays "syncing" until the
      // terminal stage event arrives (the sync is still running in the
      // background). Clearing here is what made the indicator vanish in ~4ms.
    },
    [onToast, refresh, t]
  );

  const handleBuild = useCallback(
    async (source: MemorySourceEntry) => {
      const scope = sourceTreeScope(source);
      if (!scope) return;
      setBuildingId(source.id);
      try {
        const resp = await memoryTreeFlushSource(scope);
        onToast?.({
          type: 'success',
          title: t('memorySources.build.successTitle'),
          message: `${resp.seals_fired} ${t('memorySources.build.sealsMessage')}`,
        });
      } catch (err) {
        onToast?.({
          type: 'error',
          title: t('memorySources.build.failedTitle'),
          message: err instanceof Error ? err.message : String(err),
        });
      } finally {
        setBuildingId(prev => (prev === source.id ? null : prev));
      }
    },
    [onToast, t]
  );

  const handleAdded = useCallback(
    (source: MemorySourceEntry) => {
      setSources(prev => [...prev, source]);
      onToast?.({ type: 'success', title: t('memorySources.added'), message: source.label });
      void refresh();
    },
    [onToast, refresh, t]
  );

  const handleConfirmAllIn = useCallback(async () => {
    if (allInInFlightRef.current) return;
    allInInFlightRef.current = true;
    setApplyingAllIn(true);
    try {
      const result = await applyAllIn();
      setSources(result.sources);
      // openhuman#5820: the RPC resolves even when triggers failed, so the
      // verdict comes from the counts, not from "the call did not throw".
      // Every trigger failing (the incident: `no memory source registered`
      // x4) is an error; a partial start is a warning that names both
      // counts. Only a clean sweep is a success.
      if (result.sync_failed > 0 && result.sync_triggered === 0) {
        onToast?.({
          type: 'error',
          title: t('memorySources.allIn.allFailed'),
          message: result.sync_errors[0],
        });
      } else if (result.sync_failed > 0) {
        onToast?.({
          type: 'warning',
          title: t('memorySources.allIn.partial')
            .replace('{triggered}', String(result.sync_triggered))
            .replace('{failed}', String(result.sync_failed)),
          message: result.sync_errors[0],
        });
      } else {
        onToast?.({ type: 'success', title: t('memorySources.allIn.success') });
      }
    } catch (err) {
      onToast?.({
        type: 'error',
        title: t('memorySources.allIn.failed'),
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      allInInFlightRef.current = false;
      setApplyingAllIn(false);
      setAllInModalOpen(false);
    }
  }, [onToast, t]);

  const handleSettingsSaved = useCallback((updated: MemorySourceEntry) => {
    setSources(prev => prev.map(s => (s.id === updated.id ? updated : s)));
  }, []);

  const handleToggleSettings = useCallback((sourceId: string) => {
    setExpandedSettingsId(prev => (prev === sourceId ? null : sourceId));
  }, []);

  const allInModal: ConfirmationModalType = {
    isOpen: allInModalOpen,
    title: t('memorySources.allIn.title'),
    message: t('memorySources.allIn.message'),
    confirmText: t('memorySources.allIn.confirm'),
    cancelText: t('memorySources.allIn.cancel'),
    destructive: false,
    onConfirm: () => {
      void handleConfirmAllIn();
    },
    onCancel: () => {
      setAllInModalOpen(false);
    },
  };

  return (
    <Card padded divided={false} data-testid="memory-sources">
      <header className="mb-3 flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-content-secondary">{t('memorySources.title')}</h3>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setAllInModalOpen(true)}
            disabled={applyingAllIn}
            data-testid="all-in-button"
            leadingIcon={<AllInIcon />}
            className="border-primary-300 text-primary-600 hover:bg-primary-50
                       dark:border-primary-500/30 dark:text-primary-400 dark:hover:bg-primary-500/10">
            {t('memorySources.allIn.button')}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => setDialogOpen(true)}
            leadingIcon={<PlusIcon />}>
            {t('memorySources.addSource')}
          </Button>
        </div>
      </header>

      <MemorySyncSchedule lastSyncMs={overallLastSyncMs} onToast={onToast} />

      {loading ? (
        <p className="text-xs text-content-muted">{t('common.loading')}</p>
      ) : sources.length === 0 ? (
        <p className="text-xs text-content-muted">{t('memorySources.empty')}</p>
      ) : (
        <ul className="divide-y divide-line-subtle">
          {sources.map(source => (
            <MemorySourceRow
              key={source.id}
              source={source}
              status={statusById.get(source.id) ?? null}
              pipeline={pipeline}
              isAuthenticated={isAuthenticated}
              isSyncing={syncingIds.has(source.id) || syncProgress.has(source.id)}
              isBuilding={buildingId === source.id}
              progress={syncProgress.get(source.id) ?? null}
              result={syncResults.get(source.id) ?? null}
              settingsExpanded={expandedSettingsId === source.id}
              onToggle={handleToggle}
              onRemove={handleRemove}
              onSync={handleSync}
              onBuild={handleBuild}
              onToggleSettings={handleToggleSettings}
              onSettingsSaved={handleSettingsSaved}
              onToast={onToast}
              onViewHealth={() => {
                console.debug('[ui-flow][memory-sources] view memory health from source row');
                navigate('/brain?tab=sync');
              }}
              onSignIn={() => {
                console.debug(
                  '[ui-flow][memory-sources] sign-in prompt from stored-without-vectors'
                );
                navigate('/auth');
              }}
            />
          ))}
        </ul>
      )}

      <AddMemorySourceDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        onAdded={handleAdded}
      />

      {allInModalOpen && (
        <ConfirmationModal modal={allInModal} onClose={() => setAllInModalOpen(false)} />
      )}
    </Card>
  );
}
