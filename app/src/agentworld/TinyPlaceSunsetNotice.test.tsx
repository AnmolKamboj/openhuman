import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import TinyPlaceSunsetNotice from './TinyPlaceSunsetNotice';

const openUrl = vi.fn();
vi.mock('../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
vi.mock('../utils/openUrl', () => ({ openUrl: (url: string) => openUrl(url) }));

describe('TinyPlaceSunsetNotice (#5424)', () => {
  it('renders the removal notice with a call to action', () => {
    render(<TinyPlaceSunsetNotice />);

    expect(screen.getByTestId('tinyplace-sunset-notice')).toBeInTheDocument();
    expect(screen.getByText('tinyplaceSunset.title')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'tinyplaceSunset.cta' })).toBeInTheDocument();
  });

  it('opens tiny.place in the system browser when the CTA is clicked', () => {
    render(<TinyPlaceSunsetNotice />);

    fireEvent.click(screen.getByRole('button', { name: 'tinyplaceSunset.cta' }));
    expect(openUrl).toHaveBeenCalledWith('https://tiny.place');
  });

  it('is not dismissible — no dismiss control is rendered', () => {
    render(<TinyPlaceSunsetNotice />);

    expect(screen.queryByRole('button', { name: 'common.dismiss' })).toBeNull();
  });
});
