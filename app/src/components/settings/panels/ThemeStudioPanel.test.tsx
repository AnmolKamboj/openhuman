import { fireEvent, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { renderWithProviders } from '../../../test/test-utils';
import ThemeStudioPanel from './ThemeStudioPanel';

const themeState = {
  mode: 'system',
  tabBarLabels: 'hover',
  fontSize: 'medium',
  activeThemeId: 'system',
  customThemes: [],
};

describe('<ThemeStudioPanel />', () => {
  it('renders the family gallery', () => {
    renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });
    // Theme families (each with a Light/Dark/Auto variant toggle).
    expect(screen.getByText('Classic')).toBeInTheDocument();
    expect(screen.getByText('Ocean')).toBeInTheDocument();
    expect(screen.getByText('Matrix')).toBeInTheDocument();
    expect(screen.getByText('HAL 9000')).toBeInTheDocument();
  });

  it('auto-forks a custom theme when a preset colour is edited', () => {
    const { store } = renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });

    expect(store.getState().theme.customThemes).toHaveLength(0);
    // Editing a colour on a preset transparently forks a custom theme.
    const colorInput = document.querySelector('input[type="color"]') as HTMLInputElement;
    expect(colorInput).not.toBeNull();
    fireEvent.input(colorInput, { target: { value: '#ff0000' } });

    const { customThemes, activeThemeId } = store.getState().theme;
    expect(customThemes).toHaveLength(1);
    expect(customThemes[0].builtIn).toBe(false);
    expect(activeThemeId).toBe(customThemes[0].id);
  });

  it('keeps colour editing enabled even on a preset (edits auto-fork)', () => {
    renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });
    // No disabled colour inputs — editing is always available.
    expect(document.querySelector('input[type="color"]:not([disabled])')).not.toBeNull();
    expect(document.querySelector('input[type="color"][disabled]')).toBeNull();
  });

  it('preserves gradient and backdrop settings from imported themes', () => {
    const { store } = renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });

    const imported = {
      name: 'Imported studio theme',
      isDark: true,
      colors: { surface: '1 2 3' },
      fonts: {},
      gradient: { canvas: 'linear-gradient(red, blue)' },
      backdrop: { kind: 'image', imageUrl: 'https://example.com/bg.jpg' },
    };

    fireEvent.change(screen.getByLabelText('Import theme'), {
      target: { value: JSON.stringify(imported) },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Import' }));

    expect(store.getState().theme.customThemes[0]).toMatchObject({
      name: 'Imported studio theme',
      gradient: { canvas: 'linear-gradient(red, blue)' },
      backdrop: { kind: 'image', imageUrl: 'https://example.com/bg.jpg' },
    });
  });

  // #5901: `typeof null === 'object'` and `typeof [] === 'object'`, so the old
  // shape check let both through; `{ ...null }` and `{ ...[] }` each yield `{}`,
  // and a zero-token theme was stored and could be activated from the gallery.
  // `{}` is refused for the same reason — `applyTheme` writes one custom
  // property per token, so a colourless theme applies nothing at all.
  it.each([
    ['null colors', null],
    ['an array of colors', []],
    ['an empty colors object', {}],
    ['a string colors value', 'surface'],
    ['a numeric colors value', 42],
  ])('refuses to import a theme with %s', (_label, colors) => {
    const { store } = renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });

    fireEvent.change(screen.getByLabelText('Import theme'), {
      target: { value: JSON.stringify({ name: 'Colourless', isDark: false, colors }) },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Import' }));

    expect(screen.getByText('Could not parse that theme JSON.')).toBeInTheDocument();
    expect(store.getState().theme.customThemes).toHaveLength(0);
  });

  it('still imports a theme carrying a single colour token', () => {
    const { store } = renderWithProviders(<ThemeStudioPanel />, {
      preloadedState: { theme: themeState },
      initialEntries: ['/settings/theme'],
    });

    fireEvent.change(screen.getByLabelText('Import theme'), {
      target: {
        value: JSON.stringify({ name: 'Minimal', isDark: false, colors: { surface: '1 2 3' } }),
      },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Import' }));

    expect(store.getState().theme.customThemes).toHaveLength(1);
    expect(store.getState().theme.customThemes[0]).toMatchObject({
      name: 'Minimal',
      colors: { surface: '1 2 3' },
    });
  });
});
