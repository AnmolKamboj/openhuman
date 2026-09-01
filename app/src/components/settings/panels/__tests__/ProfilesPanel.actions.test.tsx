/**
 * ProfilesPanel — the action-error paths, and the built-in/active affordance
 * rules.
 *
 * `ProfilesPanel.test.tsx` (sibling) covers the happy paths and the *load*
 * error. Neither `setActive` nor `remove` has its failure branch exercised
 * anywhere, which is how the `[object Object]` defect at `ProfilesPanel.tsx:46`
 * and `:59` survived — see `~/tinyhuman/bugs/W1-settings-bugs.md`. It also does
 * not assert which buttons a built-in or already-active profile may show; those
 * are the guards that stop a user deleting a built-in.
 *
 * Separate file rather than an edit to the sibling: three of us are in this
 * directory.
 */
import { configureStore } from '@reduxjs/toolkit';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { Provider } from 'react-redux';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { agentProfilesApi } from '../../../../services/api/agentProfilesApi';
import agentProfilesReducer from '../../../../store/agentProfileSlice';
import type { AgentProfile, AgentProfilesResponse } from '../../../../types/agentProfile';
import ProfilesPanel from '../ProfilesPanel';

vi.mock('../../../../services/api/agentProfilesApi', () => ({
  agentProfilesApi: { list: vi.fn(), select: vi.fn(), upsert: vi.fn(), delete: vi.fn() },
}));

const mockNavigate = vi.fn();
vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => mockNavigate };
});

const mockList = vi.mocked(agentProfilesApi.list);
const mockSelect = vi.mocked(agentProfilesApi.select);
const mockDelete = vi.mocked(agentProfilesApi.delete);

function profile(overrides: Partial<AgentProfile> = {}): AgentProfile {
  return {
    id: 'research',
    name: 'Research',
    description: 'Source-grounded research.',
    agentId: 'researcher',
    builtIn: true,
    ...overrides,
  };
}

function response(profiles: AgentProfile[], activeProfileId: string): AgentProfilesResponse {
  return { profiles, activeProfileId };
}

const PROFILES = [
  profile({ id: 'default', name: 'Default', builtIn: true }),
  profile({ id: 'writer', name: 'Writer', description: 'Drafts copy.', builtIn: false }),
];

function renderPanel() {
  const store = configureStore({ reducer: { agentProfiles: agentProfilesReducer } });
  render(
    <Provider store={store}>
      <MemoryRouter>
        <ProfilesPanel />
      </MemoryRouter>
    </Provider>
  );
  return store;
}

/** The <li> for a profile, addressed by its visible name. */
const row = (name: string) => screen.getByText(name).closest('li') as HTMLElement;

describe('ProfilesPanel — action failures', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockList.mockResolvedValue(response(PROFILES, 'default'));
  });

  it('shows an error when activating a profile fails', async () => {
    mockSelect.mockRejectedValueOnce(new Error('select blew up'));
    renderPanel();

    await screen.findByText('Writer');
    fireEvent.click(within(row('Writer')).getByText('Set as active'));

    await waitFor(() => expect(mockSelect).toHaveBeenCalledWith('writer'));
    // The panel must say *something* — silence would leave the user believing
    // the switch took effect.
    expect(await screen.findByText(/object Object|select blew up/)).toBeInTheDocument();
  });

  it('shows an error when deleting a profile fails', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    mockDelete.mockRejectedValueOnce(new Error('delete blew up'));
    renderPanel();

    await screen.findByText('Writer');
    fireEvent.click(within(row('Writer')).getByText('Delete'));

    await waitFor(() => expect(mockDelete).toHaveBeenCalledWith('writer'));
    expect(await screen.findByText(/object Object|delete blew up/)).toBeInTheDocument();
    confirmSpy.mockRestore();
  });

  // BUG (`~/tinyhuman/bugs/W1-settings-bugs.md` #1): both handlers stringify
  // Redux Toolkit's SerializedError — a plain object, not an `Error` — so
  // `err instanceof Error ? err.message : String(err)` (`ProfilesPanel.tsx:46`
  // and `:59`) yields "[object Object]". The message never reaches the user.
  //
  // Pinned as CURRENT behaviour. When it is fixed, assert the message here and
  // delete this comment — do not delete the test.
  it('renders both action failures as [object Object] (known defect)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    mockSelect.mockRejectedValueOnce(new Error('select blew up'));
    renderPanel();

    await screen.findByText('Writer');
    fireEvent.click(within(row('Writer')).getByText('Set as active'));

    const shown = await screen.findByText('[object Object]');
    expect(shown).toBeInTheDocument();
    expect(screen.queryByText(/select blew up/)).not.toBeInTheDocument();
    confirmSpy.mockRestore();
  });

  it('clears a previous action error when the next action succeeds', async () => {
    mockSelect.mockRejectedValueOnce(new Error('first attempt fails'));
    mockSelect.mockResolvedValueOnce(response(PROFILES, 'writer'));
    renderPanel();

    await screen.findByText('Writer');
    fireEvent.click(within(row('Writer')).getByText('Set as active'));
    expect(await screen.findByText('[object Object]')).toBeInTheDocument();

    fireEvent.click(within(row('Writer')).getByText('Set as active'));
    await waitFor(() => expect(screen.queryByText('[object Object]')).not.toBeInTheDocument());
  });

  it('does not delete, and raises no error, when the confirm is dismissed', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    renderPanel();

    await screen.findByText('Writer');
    fireEvent.click(within(row('Writer')).getByText('Delete'));

    expect(mockDelete).not.toHaveBeenCalled();
    expect(screen.queryByText('[object Object]')).not.toBeInTheDocument();
    confirmSpy.mockRestore();
  });
});

describe('ProfilesPanel — affordance rules', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockList.mockResolvedValue(response(PROFILES, 'default'));
  });

  afterEach(() => vi.restoreAllMocks());

  it('offers no Delete on a built-in profile', async () => {
    renderPanel();
    await screen.findByText('Default');

    // `Default` is builtIn; deleting a shipped profile must not be offered.
    expect(within(row('Default')).queryByText('Delete')).not.toBeInTheDocument();
    expect(within(row('Writer')).getByText('Delete')).toBeInTheDocument();
  });

  it('offers no Set active on the profile that is already active', async () => {
    renderPanel();
    await screen.findByText('Default');

    expect(within(row('Default')).queryByText('Set as active')).not.toBeInTheDocument();
    expect(within(row('Writer')).getByText('Set as active')).toBeInTheDocument();
  });

  it('always offers Edit, on built-in and custom alike', async () => {
    renderPanel();
    await screen.findByText('Default');

    expect(within(row('Default')).getByText('Edit')).toBeInTheDocument();
    expect(within(row('Writer')).getByText('Edit')).toBeInTheDocument();
  });

  it('labels the active profile and the built-in/custom source', async () => {
    renderPanel();
    await screen.findByText('Default');

    expect(within(row('Default')).getByText('Active')).toBeInTheDocument();
    expect(within(row('Writer')).queryByText('Active')).not.toBeInTheDocument();
    expect(within(row('Default')).getByText('Built-in')).toBeInTheDocument();
    expect(within(row('Writer')).getByText('Custom')).toBeInTheDocument();
  });
});
