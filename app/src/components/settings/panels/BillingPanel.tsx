import createDebug from 'debug';
import { useEffect, useState } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import { billingApi } from '../../../services/api/billingApi';
import { creditsApi, type TeamUsage } from '../../../services/api/creditsApi';
import type { BillingSummaryData } from '../../../types/api';
import { BILLING_DASHBOARD_URL } from '../../../utils/links';
import { openUrl } from '../../../utils/openUrl';
import Button from '../../ui/Button';
import { SettingsStatusLine } from '../controls';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';
import SettingsPanel from '../layout/SettingsPanel';
import InferenceBudget from './billing/InferenceBudget';

const log = createDebug('openhuman:billing:panel');
const formatUsd = (amount: number): string => `$${amount.toFixed(2)}`;

const BillingPanel = () => {
  const { t } = useT();
  const { navigateBack } = useSettingsNavigation();
  const [summary, setSummary] = useState<BillingSummaryData | null>(null);
  const [teamUsage, setTeamUsage] = useState<TeamUsage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      log('loading billing summary and cycle usage');
      const [summaryResult, usageResult] = await Promise.allSettled([
        billingApi.getSummary(),
        creditsApi.getTeamUsage(),
      ]);
      if (cancelled) return;

      if (summaryResult.status === 'fulfilled') {
        setSummary(summaryResult.value);
      } else {
        log('summary load failed error=%s', String(summaryResult.reason));
      }

      if (usageResult.status === 'fulfilled') {
        setTeamUsage(usageResult.value);
      } else {
        log('usage load failed error=%s', String(usageResult.reason));
      }

      const failure =
        summaryResult.status === 'rejected'
          ? summaryResult.reason
          : usageResult.status === 'rejected'
            ? usageResult.reason
            : null;
      if (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      }
      setLoading(false);
      log('billing state applied summary=%s usage=%s', summaryResult.status, usageResult.status);
    };

    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  // Only /teams/me/usage includes the remaining subscription-cycle allowance.
  // The summary's totalUsd is wallet-only (promotion + top-up), so it is not a
  // safe fallback for the account's true available balance.
  const availableUsd = teamUsage?.remainingUsd;
  const topUpUrl = summary?.links.topUpUrl ?? `${BILLING_DASHBOARD_URL}?tab=billing`;
  const manageUrl = summary?.links.manageUrl ?? BILLING_DASHBOARD_URL;

  return (
    <SettingsPanel>
      <SettingsStatusLine saving={loading} error={error} savingLabel={t('common.loading')} />

      <section className="rounded-2xl border border-line bg-surface p-4 space-y-4">
        <div className="space-y-1">
          <h2 className="text-base font-semibold text-content">
            {t('settings.billing.movedToWeb')}
          </h2>
          <p className="text-sm text-content-muted">{t('settings.billing.movedToWebDesc')}</p>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <SummaryTile
            label={t('settings.billing.subscription.currentPlan')}
            value={
              summary?.plan.plan ??
              (loading ? '...' : t('settings.billing.inferenceBudget.notAvailable'))
            }
          />
          <SummaryTile
            label={t('settings.billing.payAsYouGo.available')}
            value={
              availableUsd === undefined
                ? loading
                  ? '...'
                  : t('settings.billing.inferenceBudget.notAvailable')
                : formatUsd(availableUsd)
            }
          />
          <SummaryTile
            label={t('settings.billing.payAsYouGo.promotionalCredits')}
            value={
              summary
                ? formatUsd(summary.credits.promotionBalanceUsd)
                : loading
                  ? '...'
                  : t('settings.billing.inferenceBudget.notAvailable')
            }
          />
          <SummaryTile
            label={t('settings.billing.payAsYouGo.topUpBalance')}
            value={
              summary
                ? formatUsd(summary.credits.teamTopupUsd)
                : loading
                  ? '...'
                  : t('settings.billing.inferenceBudget.notAvailable')
            }
          />
        </div>
      </section>

      <InferenceBudget teamUsage={teamUsage} isLoadingCredits={loading} />

      <div className="flex flex-wrap gap-3">
        <Button type="button" variant="primary" size="md" onClick={() => void openUrl(topUpUrl)}>
          {t('settings.billing.payAsYouGo.topUpCredits')}
        </Button>
        <Button type="button" variant="secondary" size="md" onClick={() => void openUrl(manageUrl)}>
          {t('settings.billing.openDashboard')}
        </Button>
        <Button type="button" variant="tertiary" size="md" onClick={navigateBack}>
          {t('settings.billing.backToSettings')}
        </Button>
      </div>
    </SettingsPanel>
  );
};

const SummaryTile = ({ label, value }: { label: string; value: string }) => (
  <div className="rounded-xl border border-line bg-surface-muted px-3 py-3">
    <div className="text-[11px] text-content-muted">{label}</div>
    <div className="mt-1 text-lg font-semibold text-content">{value}</div>
  </div>
);

export default BillingPanel;
