/**
 * ExploreSection — Agent World "Explore" overview.
 *
 * Calls `explorer.overview()` via the invoke API client (native Rust core →
 * tiny.place SDK) and renders the network overview as stat cards inside the
 * standard `PanelScaffold` chrome, matching the rest of the app. Handles
 * loading / wallet-locked / payment / error states.
 */
import { useEffect, useState } from 'react';

import PanelScaffold from '../../components/layout/PanelScaffold';
import { type ExplorerOverview, PaymentRequiredError } from '../../lib/agentworld/invokeApiClient';
import { apiClient } from '../AgentWorldShell';

type State =
  | { status: 'loading' }
  | { status: 'payment_required'; challenge: unknown }
  | { status: 'error'; message: string }
  | { status: 'ok'; data: ExplorerOverview };

/** Defensive view of the explorer overview payload (fields are best-effort). */
interface OverviewShape {
  allTime?: { feesUsd?: string; registeredAgents?: number; volumeUsd?: string };
  last24h?: { feesUsd?: string; transactions?: number; uniqueAgents?: number; volumeUsd?: string };
  ledger?: { totalEntries?: number; latestTxId?: string; latestTimestamp?: string };
}

function useExplorerOverview(): State {
  const [state, setState] = useState<State>({ status: 'loading' });

  useEffect(() => {
    let cancelled = false;

    void apiClient.explorer
      .overview()
      .then(data => {
        if (!cancelled) setState({ status: 'ok', data });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        if (err instanceof PaymentRequiredError) {
          setState({ status: 'payment_required', challenge: err.challenge });
        } else {
          setState({ status: 'error', message: String(err) });
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return state;
}

function usd(value?: string): string {
  if (value == null) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return `$${value}`;
  return `$${n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function num(value?: number): string {
  return value == null ? '—' : value.toLocaleString();
}

function StatCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-lg border border-stone-200 bg-white p-4 dark:border-neutral-800 dark:bg-neutral-900">
      <div className="text-[11px] font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400">
        {label}
      </div>
      <div className="mt-1.5 text-2xl font-semibold text-stone-900 dark:text-neutral-100">
        {value}
      </div>
      {sub && <div className="mt-0.5 text-xs text-stone-400 dark:text-neutral-500">{sub}</div>}
    </div>
  );
}

/** Centered status message used for loading / wallet / error states. */
function StatusBlock({ tone, title, body }: { tone: string; title: string; body?: string }) {
  return (
    <div className="flex h-64 flex-col items-center justify-center gap-2 text-center">
      <p className={`text-base font-medium ${tone}`}>{title}</p>
      {body && <p className="max-w-md text-sm text-stone-500 dark:text-neutral-400">{body}</p>}
    </div>
  );
}

export default function ExploreSection() {
  const state = useExplorerOverview();

  let body: React.ReactNode;

  if (state.status === 'loading') {
    body = (
      <div className="flex h-64 items-center justify-center text-stone-400 dark:text-neutral-500">
        <span className="animate-pulse text-sm">Loading network overview…</span>
      </div>
    );
  } else if (state.status === 'payment_required') {
    body = (
      <StatusBlock
        tone="text-amber-600 dark:text-amber-400"
        title="Access requires payment"
        body="Your wallet will be used to fulfill the x402 payment challenge."
      />
    );
  } else if (state.status === 'error') {
    const isWalletLocked =
      state.message.includes('wallet is not configured') ||
      state.message.includes('wallet secret material is missing');
    body = isWalletLocked ? (
      <StatusBlock
        tone="text-stone-700 dark:text-neutral-200"
        title="Unlock your wallet to use Agent World"
        body="Agent World uses your wallet identity. Import your recovery phrase in Settings to continue."
      />
    ) : (
      <StatusBlock
        tone="text-red-600 dark:text-red-400"
        title="Failed to load Agent World"
        body={state.message}
      />
    );
  } else {
    const ov = state.data as unknown as OverviewShape;
    body = (
      <>
        <div>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400">
            All time
          </h3>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <StatCard label="Registered agents" value={num(ov.allTime?.registeredAgents)} />
            <StatCard label="Volume" value={usd(ov.allTime?.volumeUsd)} />
            <StatCard label="Fees" value={usd(ov.allTime?.feesUsd)} />
          </div>
        </div>
        <div>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400">
            Last 24 hours
          </h3>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <StatCard label="Transactions" value={num(ov.last24h?.transactions)} />
            <StatCard label="Active agents" value={num(ov.last24h?.uniqueAgents)} />
            <StatCard label="Volume" value={usd(ov.last24h?.volumeUsd)} />
            <StatCard label="Fees" value={usd(ov.last24h?.feesUsd)} />
          </div>
        </div>
        <div>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400">
            Ledger
          </h3>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <StatCard label="Total entries" value={num(ov.ledger?.totalEntries)} />
            <StatCard
              label="Latest tx"
              value={ov.ledger?.latestTxId ?? '—'}
              sub={
                ov.ledger?.latestTimestamp
                  ? new Date(ov.ledger.latestTimestamp).toLocaleString()
                  : undefined
              }
            />
          </div>
        </div>
      </>
    );
  }

  return <PanelScaffold description="Network overview">{body}</PanelScaffold>;
}
