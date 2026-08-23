import { Dialog as DialogPrimitive } from 'radix-ui';
import { type ReactNode } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import Button from '../../ui/Button';
import { DialogOverlay, DialogRoot } from '../../ui/Dialog';
import { CloseIcon } from '../../ui/icons';
import VisuallyHidden from '../../ui/VisuallyHidden';

interface SettingsModalFrameProps {
  /** Invoked on X click, Esc, or backdrop click. */
  onClose: () => void;
  children: ReactNode;
  /** id of the element labelling the dialog, if any. */
  labelledBy?: string;
}

/**
 * Presentational chrome for the desktop Settings modal: a portalled, dimmed
 * backdrop and a centered, full-app-size card with a floating close button.
 *
 * Backed by Radix `Dialog` (see `ui/ModalShell.tsx` for the rationale — real
 * focus trap, scroll lock, `aria-hidden` on the rest of the tree, and focus
 * restore on unmount). This frame does not reuse `ModalShell` itself: it needs
 * no title bar, a two-column body instead of a single content slot, and a
 * close affordance floated *above* the card rather than inside a header row —
 * so it composes the lower-level `DialogRoot`/`DialogOverlay`/
 * `DialogPrimitive.Content` pieces directly.
 */
export function SettingsModalFrame({ onClose, children, labelledBy }: SettingsModalFrameProps) {
  const { t } = useT();

  // Portal into #root (not document.body) so the modal stays inside the app's
  // tested subtree — `#root`-scoped checks (and E2E specs reading
  // `#root.innerText()`) see the routed panel. Falls back to body if absent.
  const portalTarget = document.getElementById('root') ?? document.body;

  return (
    <DialogRoot
      open
      onOpenChange={next => {
        if (!next) onClose();
      }}>
      <DialogPrimitive.Portal container={portalTarget}>
        <DialogOverlay
          data-testid="settings-modal-backdrop"
          className="flex items-center justify-center"
        />
        {/* Positioning wrapper sized to the card, rendered as a sibling of the
            overlay so it sits above the scrim without needing its own click
            handler — Radix's dismissable layer already treats a click on the
            overlay as an outside interaction. The close button is a sibling of
            the card so it can float just above the top-right corner — outside
            the card surface — and never overlap panel content. No overflow
            clip here. */}
        <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center">
          <div className="pointer-events-auto relative mx-4 flex h-[80vh] w-full max-w-5xl">
            <Button
              variant="secondary"
              iconOnly
              size="sm"
              aria-label={t('common.close')}
              data-testid="settings-modal-close"
              onClick={onClose}
              className="absolute bottom-full right-0 mb-2 h-8 w-8 rounded-full text-content-muted shadow-md hover:text-content-secondary">
              <CloseIcon className="h-4 w-4" />
            </Button>
            <DialogPrimitive.Content
              aria-labelledby={labelledBy}
              aria-label={labelledBy ? undefined : t('nav.settings')}
              data-testid="settings-modal-card"
              // Radius matches the shell's ContentSurface so the modal reads as the
              // app's one content card lifted forward. It keeps `shadow-xl` rather
              // than the shell's `shadow-content-edge` hairline: this card floats on
              // a dimmed scrim with no adjacent chrome to seam against, and the two
              // utilities would collide on `box-shadow` anyway.
              className="flex h-full w-full overflow-hidden rounded-2xl bg-surface shadow-xl animate-fade-up focus:outline-hidden">
              {/* Radix requires a Title in the tree for the dialog to be
                  accessible. When the caller supplies its own visible heading
                  via `labelledBy`, that id still wins as the accessible name
                  (set above); this hidden node only satisfies Radix's a11y
                  check and gives screen readers a fallback name otherwise. */}
              <VisuallyHidden>
                <DialogPrimitive.Title>{t('nav.settings')}</DialogPrimitive.Title>
              </VisuallyHidden>
              {children}
            </DialogPrimitive.Content>
          </div>
        </div>
      </DialogPrimitive.Portal>
    </DialogRoot>
  );
}
