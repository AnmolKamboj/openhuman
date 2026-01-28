import { HashRouter as Router } from 'react-router-dom';
import { Provider } from 'react-redux';
import { PersistGate } from 'redux-persist/integration/react';
import { store, persistor } from './store';
import SocketProvider from './providers/SocketProvider';
import TelegramProvider from './providers/TelegramProvider';
import AppRoutes from './AppRoutes';

function App() {
  return (
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <SocketProvider>
          <TelegramProvider>
            <Router>
              <AppRoutes />
            </Router>
          </TelegramProvider>
        </SocketProvider>
      </PersistGate>
    </Provider>
  );
}

export default App;
