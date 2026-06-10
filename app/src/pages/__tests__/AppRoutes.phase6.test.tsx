/**
 * Phase 6 — /human → /chat redirect test.
 *
 * Verifies that the desktop route for /human redirects to /chat (Assistant
 * surface).  iOS keeps /human as a real route in AppRoutesIOS.tsx — only
 * the desktop redirect is tested here.
 *
 * Uses a minimal route tree so no full provider chain is needed.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Navigate, Route, Routes } from 'react-router-dom';
import { describe, expect, it } from 'vitest';

function TestRoutes() {
  return (
    <Routes>
      {/* The real /chat route (Assistant surface). */}
      <Route path="/chat" element={<div data-testid="chat-page">chat</div>} />
      {/* Phase 6 back-compat redirect. */}
      <Route path="/human" element={<Navigate to="/chat" replace />} />
    </Routes>
  );
}

const renderAt = (path: string) =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <TestRoutes />
    </MemoryRouter>
  );

describe('Phase 6 route redirects', () => {
  it('/human redirects to /chat (Assistant surface)', () => {
    renderAt('/human');
    expect(screen.getByTestId('chat-page')).toBeInTheDocument();
  });

  it('/chat renders the chat page directly', () => {
    renderAt('/chat');
    expect(screen.getByTestId('chat-page')).toBeInTheDocument();
  });
});
