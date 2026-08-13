import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { TinyPlaceIdentityState } from '../hooks/useTinyPlaceIdentity';
import AgentWorldShell from './AgentWorldShell';

let identity: TinyPlaceIdentityState = { status: 'ready', hasIdentity: true };
vi.mock('../hooks/useTinyPlaceIdentity', () => ({ useTinyPlaceIdentity: () => identity }));
vi.mock('./TinyPlaceSunsetNotice', () => ({
  default: () => <div data-testid="tinyplace-sunset-notice" />,
}));
vi.mock('../lib/agentworld/invokeApiClient', () => ({ createInvokeApiClient: () => ({}) }));

function renderShell() {
  return render(
    <MemoryRouter initialEntries={['/agent-world']}>
      <Routes>
        <Route
          path="/agent-world"
          element={
            <AgentWorldShell>
              <div data-testid="agent-world-content" />
            </AgentWorldShell>
          }
        />
        <Route path="/chat" element={<div data-testid="chat-page" />} />
      </Routes>
    </MemoryRouter>
  );
}

describe('AgentWorldShell tiny.place gate (#5424)', () => {
  beforeEach(() => {
    identity = { status: 'ready', hasIdentity: true };
  });

  it('renders the agent-world surface and the notice for an identity holder', () => {
    identity = { status: 'ready', hasIdentity: true };
    renderShell();

    expect(screen.getByTestId('agent-world-content')).toBeInTheDocument();
    expect(screen.getByTestId('tinyplace-sunset-notice')).toBeInTheDocument();
    expect(screen.queryByTestId('chat-page')).toBeNull();
  });

  it('redirects a confirmed non-holder away to chat', () => {
    identity = { status: 'ready', hasIdentity: false };
    renderShell();

    expect(screen.getByTestId('chat-page')).toBeInTheDocument();
    expect(screen.queryByTestId('agent-world-content')).toBeNull();
  });

  it('renders optimistically while the identity check is still loading', () => {
    identity = { status: 'loading', hasIdentity: false };
    renderShell();

    // A holder must not see a flash-then-redirect, so nothing redirects until
    // the check confirms the user has no identity.
    expect(screen.getByTestId('agent-world-content')).toBeInTheDocument();
    expect(screen.queryByTestId('chat-page')).toBeNull();
  });
});
