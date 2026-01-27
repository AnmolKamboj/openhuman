import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Provider } from 'react-redux';
import { PersistGate } from 'redux-persist/integration/react';
import { store, persistor } from './store';
import Welcome from './pages/Welcome';
import Login from './pages/Login';
import Onboarding from './pages/onboarding/Onboarding';
import Home from './pages/Home';
import PublicRoute from './components/PublicRoute';
import ProtectedRoute from './components/ProtectedRoute';
import DefaultRedirect from './components/DefaultRedirect';

function App() {
  return (
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <Router>
          <Routes>
            {/* Public routes - redirect to /home or /onboarding if logged in */}
            <Route
              path="/"
              element={
                <PublicRoute>
                  <Welcome />
                </PublicRoute>
              }
            />
            <Route
              path="/login"
              element={
                <PublicRoute>
                  <Login />
                </PublicRoute>
              }
            />

            {/* Protected routes */}
            <Route
              path="/onboarding"
              element={
                <ProtectedRoute requireAuth={true} requireOnboarded={false}>
                  <Onboarding />
                </ProtectedRoute>
              }
            />
            <Route
              path="/home"
              element={
                <ProtectedRoute requireAuth={true} requireOnboarded={true} redirectTo="/onboarding">
                  <Home />
                </ProtectedRoute>
              }
            />

            {/* Default redirect based on auth status */}
            <Route path="*" element={<DefaultRedirect />} />
          </Routes>
        </Router>
      </PersistGate>
    </Provider>
  );
}

export default App;
