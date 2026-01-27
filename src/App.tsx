import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import Welcome from './pages/Welcome';
import Login from './pages/Login';
import Step1Privacy from './pages/onboarding/Step1Privacy';
import Step2Analytics from './pages/onboarding/Step2Analytics';
import Step3Connect from './pages/onboarding/Step3Connect';
import Step4GetStarted from './pages/onboarding/Step4GetStarted';
import Home from './pages/Home';

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" element={<Welcome />} />
        <Route path="/login" element={<Login />} />
        <Route path="/onboarding/step1" element={<Step1Privacy />} />
        <Route path="/onboarding/step2" element={<Step2Analytics />} />
        <Route path="/onboarding/step3" element={<Step3Connect />} />
        <Route path="/onboarding/step4" element={<Step4GetStarted />} />
        <Route path="/home" element={<Home />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Router>
  );
}

export default App;