import { Routes, Route } from 'react-router-dom';
import { useLocation } from 'react-router-dom';
import SettingsLayout from './SettingsLayout';
import SettingsHome from './SettingsHome';
import ConnectionsPanel from './panels/ConnectionsPanel';
import { useSettingsNavigation } from './hooks/useSettingsNavigation';

const SettingsModal = () => {
  const location = useLocation();
  const { closeSettings } = useSettingsNavigation();

  // Only render modal when on settings routes
  const isSettingsRoute = location.pathname.startsWith('/settings');

  if (!isSettingsRoute) {
    return null;
  }

  return (
    <SettingsLayout onClose={closeSettings}>
      <Routes>
        <Route path="/settings" element={<SettingsHome />} />
        <Route path="/settings/connections" element={<ConnectionsPanel />} />

        {/* Future settings panels */}
        <Route path="/settings/messaging" element={<div className="p-6 text-center">Messaging settings coming soon</div>} />
        <Route path="/settings/privacy" element={<div className="p-6 text-center">Privacy settings coming soon</div>} />
        <Route path="/settings/profile" element={<div className="p-6 text-center">Profile settings coming soon</div>} />
        <Route path="/settings/advanced" element={<div className="p-6 text-center">Advanced settings coming soon</div>} />
        <Route path="/settings/billing" element={<div className="p-6 text-center">Billing settings coming soon</div>} />
      </Routes>
    </SettingsLayout>
  );
};

export default SettingsModal;