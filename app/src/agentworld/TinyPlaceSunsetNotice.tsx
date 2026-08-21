/**
 * tiny.place removal notice (#5424).
 *
 * Shown on the tiny.place surfaces (Agent World + the Brain orchestration
 * sub-tab) to users who have an identity — the only people who still see the
 * feature. It tells them to keep using tiny.place at tiny.place, names the
 * 31 August 2026 in-app removal date, and links out. Non-dismissible: the
 * deadline is fixed, so the notice stays until then.
 */
import UpsellBanner from '../components/upsell/UpsellBanner';
import { useT } from '../lib/i18n/I18nContext';
import { TINYPLACE_URL } from '../utils/links';
import { openUrl } from '../utils/openUrl';

export default function TinyPlaceSunsetNotice() {
  const { t } = useT();

  return (
    <div className="relative z-20" data-testid="tinyplace-sunset-notice">
      <UpsellBanner
        variant="info"
        title={t('tinyplaceSunset.title')}
        message={t('tinyplaceSunset.message')}
        ctaLabel={t('tinyplaceSunset.cta')}
        rounded={false}
        dismissible={false}
        onCtaClick={() => {
          void openUrl(TINYPLACE_URL);
        }}
      />
    </div>
  );
}
