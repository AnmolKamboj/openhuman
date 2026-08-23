import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { listProviderModels } from '../../../../services/api/aiSettingsApi';
import { ProviderModelPickerDialog } from './ProviderModelPickerDialog';

vi.mock('../../../../services/api/aiSettingsApi', () => ({ listProviderModels: vi.fn() }));

describe('ProviderModelPickerDialog', () => {
  it('returns the provider-reported context window with a catalog selection', async () => {
    vi.mocked(listProviderModels).mockResolvedValue([
      { id: 'gpt-4o-mini', owned_by: 'openai', context_window: 128_000 },
    ]);
    const onSelect = vi.fn();

    render(
      <ProviderModelPickerDialog
        cloudProviders={[
          {
            id: 'openai',
            slug: 'openai',
            label: 'OpenAI',
            endpoint: 'https://api.openai.com/v1',
            authStyle: 'bearer',
            maskedKey: '••••',
          },
        ]}
        localModels={[]}
        ollamaRunning={false}
        claudeCodeEnabled={false}
        initial={null}
        onClose={() => {}}
        onSelect={onSelect}
      />
    );

    fireEvent.change(await screen.findByRole('combobox', { name: 'Model' }), {
      target: { value: 'gpt-4o-mini' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Use this model' }));

    await waitFor(() =>
      expect(onSelect).toHaveBeenCalledWith({
        source: { kind: 'cloud', providerSlug: 'openai' },
        model: 'gpt-4o-mini',
        contextWindow: 128_000,
      })
    );
  });
});
