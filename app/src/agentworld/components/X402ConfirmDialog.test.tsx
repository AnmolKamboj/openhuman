/**
 * Tests for X402ConfirmDialog — the confirm-before-spend dialog reused by all
 * Agent World x402 write flows. Covers the pure formatting helpers plus the
 * render branches (amount/balance display, insufficient-balance gating, busy
 * state, confirm/cancel callbacks).
 *
 * All addresses / amounts are GENERIC placeholders.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';

import X402ConfirmDialog, {
  formatUnits,
  isInsufficient,
  type X402WalletBalance,
} from './X402ConfirmDialog';

const BALANCE: X402WalletBalance = {
  raw: '50000000',
  formatted: '50',
  decimals: 6,
  assetSymbol: 'USDC',
};

function baseProps() {
  return {
    title: 'Register @placeholder',
    amount: '10000000', // 10 USDC
    asset: 'USDC',
    network: 'solana-devnet',
    balance: BALANCE,
    walletAddress: 'WaLLetdeadbeef0123456789',
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
  };
}

describe('formatUnits', () => {
  test('formats base units with decimals and trims trailing zeros', () => {
    expect(formatUnits('10000000', 6)).toBe('10');
    expect(formatUnits('10500000', 6)).toBe('10.5');
    expect(formatUnits('1', 6)).toBe('0.000001');
    expect(formatUnits('0', 6)).toBe('0');
  });

  test('returns the raw value when decimals <= 0', () => {
    expect(formatUnits('42', 0)).toBe('42');
  });
});

describe('isInsufficient', () => {
  test('true only when balance is provably below the amount', () => {
    expect(isInsufficient(BALANCE, '10000000')).toBe(false);
    expect(isInsufficient(BALANCE, '60000000')).toBe(true);
    // Unknown balance → not blocked (backend remains the gate).
    expect(isInsufficient(null, '60000000')).toBe(false);
    // Unparseable raw → not blocked.
    expect(isInsufficient({ ...BALANCE, raw: 'nope' }, '1')).toBe(false);
  });
});

describe('X402ConfirmDialog', () => {
  test('renders amount, asset, network, balance and a truncated wallet', () => {
    render(<X402ConfirmDialog {...baseProps()} />);
    expect(screen.getByTestId('x402-amount')).toHaveTextContent('10 USDC');
    expect(screen.getByTestId('x402-balance')).toHaveTextContent('50 USDC');
    expect(screen.getByText('Solana (devnet)')).toBeInTheDocument();
    expect(screen.getByText('WaLLet…6789')).toBeInTheDocument();
  });

  test('renders a friendly network label (never the raw CAIP-2 genesis hash)', () => {
    // tiny.place reports the mainnet genesis on every cluster — must collapse to
    // "Solana", not show the raw "solana:5eykt4…" hash.
    render(
      <X402ConfirmDialog {...baseProps()} network="solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp" />
    );
    expect(screen.getByText('Solana')).toBeInTheDocument();
    expect(screen.queryByText(/5eykt4/)).not.toBeInTheDocument();
  });

  test('calls onConfirm / onCancel', async () => {
    const props = baseProps();
    render(<X402ConfirmDialog {...props} />);
    await userEvent.click(screen.getByTestId('x402-confirm'));
    expect(props.onConfirm).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(props.onCancel).toHaveBeenCalledTimes(1);
  });

  test('disables confirm and shows a notice when balance is insufficient', () => {
    render(<X402ConfirmDialog {...baseProps()} amount="60000000" />);
    expect(screen.getByTestId('x402-confirm')).toBeDisabled();
    expect(screen.getByTestId('x402-insufficient')).toBeInTheDocument();
  });

  test('shows "Unknown" balance and still allows confirm when balance is null', () => {
    render(<X402ConfirmDialog {...baseProps()} balance={null} />);
    expect(screen.getByTestId('x402-balance')).toHaveTextContent('Unknown');
    expect(screen.getByTestId('x402-confirm')).toBeEnabled();
  });

  test('busy state shows the busy label and disables both actions', () => {
    render(<X402ConfirmDialog {...baseProps()} busy busyLabel="Paying…" />);
    const confirm = screen.getByTestId('x402-confirm');
    expect(confirm).toHaveTextContent('Paying…');
    expect(confirm).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled();
  });

  test('Escape while busy does not cancel (close is a no-op mid-payment)', async () => {
    const props = baseProps();
    render(<X402ConfirmDialog {...props} busy busyLabel="Paying…" />);
    await userEvent.keyboard('{Escape}');
    expect(props.onCancel).not.toHaveBeenCalled();
  });
});
