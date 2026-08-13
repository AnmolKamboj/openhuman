import debugFactory from 'debug';
import type { ReactNode } from 'react';

const log = debugFactory('shell:content-surface');

/** Shared flex/overflow behaviour — identical in both framed and unframed modes. */
const BASE = 'relative z-10 flex min-h-0 flex-1 flex-col overflow-hidden bg-surface';

/**
 * Inset, rounded card floating on the window chrome. Even 12px margin on all
 * four sides so the chrome reads as a deliberate frame rather than a seam. The
 * top gap lands larger than 12px in practice because the window-drag band
 * ({@link WindowDragBar}) sits above the card inside the same column.
 */
const FRAMED = `${BASE} m-3 rounded-2xl shadow-content-edge`;

interface ContentSurfaceProps {
  children: ReactNode;
  /**
   * Render edge-to-edge with square corners and no card seam.
   *
   * Written for native CEF provider webviews: `WebviewHost` handed the Rust
   * side a plain `{x, y, width, height}` rectangle and CEF composited that
   * child view *above* the whole HTML layer, so its corners could not be masked
   * by `overflow-hidden`, a CSS radius, or any HTML overlay — a framed card
   * under a live webview showed four square corners punching through.
   *
   * That surface was removed upstream (`WebviewHost` is gone), so **nothing
   * sets this today**. Kept because the constraint recurs for any content the
   * compositor draws above the HTML layer, and because a full-bleed page is a
   * reasonable thing to want from a layout primitive.
   */
  unframed?: boolean;
}

/**
 * The app's single content surface: the opaque sheet that routed pages render
 * onto, floating on the themed chrome layer ({@link AppBackground}) that the
 * sidebar also sits on.
 *
 * This is the "card" half of the two-layer shell. The chrome carries the
 * theme's hue; this surface stays neutral so page content reads against a
 * consistent background across every theme.
 */
export default function ContentSurface({ children, unframed = false }: ContentSurfaceProps) {
  log('render: unframed=%s', unframed);
  return (
    <div
      className={unframed ? BASE : FRAMED}
      data-testid="app-content-surface"
      data-unframed={unframed ? 'true' : undefined}>
      {children}
    </div>
  );
}
