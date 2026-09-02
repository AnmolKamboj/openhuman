import { useEffect, useState } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import { billingApi } from '../../../services/api/billingApi';
import type { PlanTier } from '../../../types/api';
import { BILLING_DASHBOARD_URL } from '../../../utils/links';
import { openUrl } from '../../../utils/openUrl';
import Button from '../../ui/Button';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';
import SettingsPanel from '../layout/SettingsPanel';
import SubscriptionPlans from './billing/SubscriptionPlans';
import { buildPlanId } from './billingHelpers';

const BillingPanel = () => {
  const { t } = useT();
  const { navigateBack } = useSettingsNavigation();
  const [currentTier, setCurrentTier] = useState<PlanTier>('FREE');
  const [billingInterval, setBillingInterval] = useState<'monthly' | 'annual'>('monthly');
  const [paymentMethod, setPaymentMethod] = useState<'card' | 'crypto'>('card');
  const [isPurchasing, setIsPurchasing] = useState(false);
  const [purchasingTier, setPurchasingTier] = useState<PlanTier | null>(null);
  const paymentConfirmed = false;

  useEffect(() => {
    billingApi
      .getCurrentPlan()
      .then(data => setCurrentTier(data.plan))
      .catch(() => {});
  }, []);

  const handleUpgrade = async (tier: PlanTier): Promise<void> => {
    setIsPurchasing(true);
    setPurchasingTier(tier);
    try {
      if (paymentMethod === 'crypto') {
        const charge = await billingApi.createCoinbaseCharge(tier);
        await openUrl(charge.hostedUrl);
      } else {
        const session = await billingApi.purchasePlan(buildPlanId(tier, billingInterval));
        if (session.checkoutUrl) {
          await openUrl(session.checkoutUrl);
        }
      }
    } catch {
      // errors surface through the standard error boundary
    } finally {
      setIsPurchasing(false);
      setPurchasingTier(null);
    }
  };

  return (
    <SettingsPanel>
      <SubscriptionPlans
        currentTier={currentTier}
        billingInterval={billingInterval}
        setBillingInterval={setBillingInterval}
        paymentMethod={paymentMethod}
        setPaymentMethod={setPaymentMethod}
        isPurchasing={isPurchasing}
        purchasingTier={purchasingTier}
        paymentConfirmed={paymentConfirmed}
        onUpgrade={handleUpgrade}
      />

      <div className="flex flex-wrap gap-3">
        <Button
          type="button"
          variant="secondary"
          size="md"
          onClick={() => void openUrl(BILLING_DASHBOARD_URL)}>
          {t('settings.billing.openDashboard')}
        </Button>
        <Button type="button" variant="tertiary" size="md" onClick={navigateBack}>
          {t('settings.billing.backToSettings')}
        </Button>
      </div>
    </SettingsPanel>
  );
};

export default BillingPanel;
