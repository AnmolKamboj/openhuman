import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import Welcome from './pages/Welcome';
import Login from './pages/Login';
import Step1Phone from './pages/onboarding/Step1Phone';
import Step2Privacy from './pages/onboarding/Step2Privacy';
import Step3Analytics from './pages/onboarding/Step3Analytics';
import Step4Connect from './pages/onboarding/Step4Connect';
import Home from './pages/Home';

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" element={<Welcome />} />
        <Route path="/login" element={<Login />} />
        <Route path="/onboarding/step1" element={<Step1Phone />} />
        <Route path="/onboarding/step2" element={<Step2Privacy />} />
        <Route path="/onboarding/step3" element={<Step3Analytics />} />
        <Route path="/onboarding/step4" element={<Step4Connect />} />
        <Route path="/home" element={<Home />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Router>
  );
}

export default App;