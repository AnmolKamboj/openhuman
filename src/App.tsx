import { HashRouter as Router } from "react-router-dom";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";
import { store, persistor } from "./store";
import UserProvider from "./providers/UserProvider";
import SocketProvider from "./providers/SocketProvider";
import TelegramProvider from "./providers/TelegramProvider";
import AppRoutes from "./AppRoutes";

function App() {
  return (
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <UserProvider>
          <SocketProvider>
            <TelegramProvider>
              <Router>
                <AppRoutes />
              </Router>
            </TelegramProvider>
          </SocketProvider>
        </UserProvider>
      </PersistGate>
    </Provider>
  );
}

export default App;
