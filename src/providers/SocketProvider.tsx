import { useEffect, useRef } from 'react';
import { useAppSelector } from '../store/hooks';
import { socketService } from '../services/socketService';

/**
 * SocketProvider manages the socket connection based on JWT token
 * - Connects when token is set
 * - Disconnects when token is unset
 */
const SocketProvider = ({ children }: { children: React.ReactNode }) => {
  const token = useAppSelector((state) => state.auth.token);
  const previousTokenRef = useRef<string | null>(null);

  useEffect(() => {
    const previousToken = previousTokenRef.current;

    // Token was set - connect
    if (token && token !== previousToken) {
      socketService.connect(token);
      previousTokenRef.current = token;
    }

    // Token was unset - disconnect
    if (!token && previousToken) {
      socketService.disconnect();
      previousTokenRef.current = null;
    }
  }, [token]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      socketService.disconnect();
    };
  }, []);

  return <>{children}</>;
};

export default SocketProvider;
