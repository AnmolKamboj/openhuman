import { isMac } from '../../../lib/commands/shortcut';
import { isTauri } from '../../../utils/tauriCommands/common';

/**
 * Height (px) of the drag strip. Matches the macOS traffic-light zone so the
 * native window controls sit within the band.
 */
export const WINDOW_DRAG_BAR_HEIGHT = 28;

/**
 * Transparent macOS window-drag band for the overlay title bar.
 *
 * The main window runs with `titleBarStyle: "Overlay"` + `hiddenTitle` (see
 * `app/src-tauri/tauri.conf.json`), so macOS draws transparent traffic lights
 * over the web content but does NOT make the top draggable on its own — the
 * webview captures the pointer events. We opt back in with a `data-tauri-drag-
 * region` band.
 *
 * Rendered **in flow** at the top of the content column ({@link
 * RootShellLayout}), above the inset {@link ContentSurface}. It reserves the
 * band the traffic lights occupy so they sit on bare window chrome rather than
 * on the content card.
 *
 * It used to be an absolutely-positioned overlay painted on top of the routed
 * view, because the content pane was edge-to-edge and any reserved inset would
 * have pushed full-bleed surfaces (the Tiny Place world canvas, the Chat
 * backdrop) down and revealed the app background above them. The two-layer
 * shell makes that moot: the card is inset by design, so an in-flow band is now
 * both simpler and correct — and it no longer steals pointer events from the
 * top ~28px of page content.
 *
 * Native CEF provider webviews composite above all HTML and so can't be dragged
 * through; that's a platform limit, not this band. The sidebar is intentionally
 * excluded — its header already drags in place.
 *
 * macOS-only: Windows/Linux keep their native decorated title bar (the
 * `Overlay` style is a no-op there), so reserving a band would only waste
 * vertical space. Outside the Tauri runtime (browser/iOS) there is no window to
 * drag, so it renders nothing.
 */
export default function WindowDragBar() {
  if (!isTauri() || !isMac()) return null;
  return (
    <div
      data-tauri-drag-region
      aria-hidden="true"
      className="flex-none"
      style={{ height: WINDOW_DRAG_BAR_HEIGHT }}
    />
  );
}
