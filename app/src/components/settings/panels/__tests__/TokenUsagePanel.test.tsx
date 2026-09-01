import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import * as tokenjuice from '../../../../utils/tauriCommands/tokenjuice';
import TokenUsagePanel from '../TokenUsagePanel';

vi.mock('../../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));

vi.mock('../../../../utils/tauriCommands/tokenjuice', async () => {
  const actual = await vi.importActual<typeof import('../../../../utils/tauriCommands/tokenjuice')>(
    '../../../../utils/tauriCommands/tokenjuice'
  );
  return {
    ...actual,
    getTokenjuiceSettings: vi.fn(),
    getTokenjuiceSavings: vi.fn(),
    updateTokenjuiceSettings: vi.fn(),
    resetTokenjuiceSavings: vi.fn(),
  };
});

const mockGetSettings = vi.mocked(tokenjuice.getTokenjuiceSettings);
const mockGetSavings = vi.mocked(tokenjuice.getTokenjuiceSavings);
const mockUpdate = vi.mocked(tokenjuice.updateTokenjuiceSettings);

const stubSettings: tokenjuice.TokenjuiceSettings = {
  router_enabled: true,
  ccr_enabled: false,
  ccr_disk_enabled: true,
  max_cache_entries: 100,
  max_cache_bytes: 1024,
  ccr_ttl_secs: null,
  min_bytes_to_compress: 512,
  ccr_min_tokens: 1000,
  search_enabled: true,
  code_enabled: false,
  html_enabled: true,
  ml_compression_enabled: false,
  ml_model_id: 'model',
  ml_target_ratio: 0.5,
  ml_sidecar_idle_timeout_secs: 30,
  ml_max_input_chars: 4096,
  ml_device: 'cpu',
};

const stubSavings: tokenjuice.SavingsStats = {
  attributionModel: 'gpt-4',
  total: { events: 0, originalTokens: 0, compactedTokens: 0, tokensSaved: 0, costSavedUsd: 0 },
  byModel: {},
  byCompressor: {},
  cache: { entries: 0, bytes: 0 },
};

describe('TokenUsagePanel', () => {
  beforeEach(() => {
    mockGetSettings.mockReset();
    mockGetSavings.mockReset();
    mockUpdate.mockReset();
    mockGetSavings.mockResolvedValue(stubSavings);
  });

  describe('when settings load succeeds', () => {
    beforeEach(() => {
      mockGetSettings.mockResolvedValue(stubSettings);
    });

    it('enables all switches and the CCR min-tokens field once settings load', async () => {
      render(<TokenUsagePanel embedded />);

      // Wait for the async load to complete — all 7 switches must be enabled.
      await waitFor(() => {
        const switches = screen.getAllByRole('switch');
        expect(switches).toHaveLength(7);
        for (const sw of switches) expect(sw).not.toBeDisabled();
      });
      expect(
        screen.getByRole('spinbutton', { name: 'settings.tokenUsage.ccrMinTokens' })
      ).not.toBeDisabled();
    });

    it('reflects the loaded toggle values', async () => {
      render(<TokenUsagePanel embedded />);

      // router_enabled=true, search_enabled=true, code_enabled=false in stubSettings.
      await waitFor(() =>
        expect(
          screen.getByRole('switch', { name: 'settings.tokenUsage.routerEnabled' })
        ).toBeChecked()
      );
      expect(screen.getByRole('switch', { name: 'settings.tokenUsage.search' })).toBeChecked();
      expect(screen.getByRole('switch', { name: 'settings.tokenUsage.code' })).not.toBeChecked();
    });

    it('calls patch when a switch is toggled', async () => {
      const user = userEvent.setup();
      mockUpdate.mockResolvedValue({ ...stubSettings, router_enabled: false });
      render(<TokenUsagePanel embedded />);

      const sw = await screen.findByRole('switch', { name: 'settings.tokenUsage.routerEnabled' });
      await user.click(sw);
      expect(mockUpdate).toHaveBeenCalledWith({ router_enabled: false });
    });

    it('keeps controls enabled when only savings fails', async () => {
      mockGetSavings.mockRejectedValue(new Error('savings rpc down'));
      render(<TokenUsagePanel embedded />);

      // Settings loaded OK — all 7 switches and the number field must still become enabled.
      await waitFor(() => {
        const switches = screen.getAllByRole('switch');
        expect(switches).toHaveLength(7);
        for (const sw of switches) expect(sw).not.toBeDisabled();
      });
      expect(
        screen.getByRole('spinbutton', { name: 'settings.tokenUsage.ccrMinTokens' })
      ).not.toBeDisabled();
    });
  });

  describe('when settings load fails', () => {
    beforeEach(() => {
      mockGetSettings.mockRejectedValue(new Error('rpc down'));
    });

    it('disables all switches and the CCR min-tokens field while settings are unavailable', async () => {
      render(<TokenUsagePanel embedded />);

      // Wait directly for the disabled state — this proves the rejection has
      // settled and React has re-rendered with settings === null.
      await waitFor(() => {
        const switches = screen.getAllByRole('switch');
        expect(switches).toHaveLength(7);
        for (const sw of switches) expect(sw).toBeDisabled();
      });
      expect(
        screen.getByRole('spinbutton', { name: 'settings.tokenUsage.ccrMinTokens' })
      ).toBeDisabled();
    });

    it('does not call patch when a disabled switch is clicked', async () => {
      const user = userEvent.setup();
      render(<TokenUsagePanel embedded />);

      const sw = await waitFor(() => {
        const el = screen.getByRole('switch', { name: 'settings.tokenUsage.routerEnabled' });
        expect(el).toBeDisabled();
        return el;
      });
      await user.click(sw);
      expect(mockUpdate).not.toHaveBeenCalled();
    });
  });
});
