import { useEffect } from 'react';
import { useAppSelector, useAppDispatch } from '../store/hooks';
import { fetchCurrentUser } from '../store/userSlice';

/**
 * UserProvider automatically fetches user data when JWT token is available
 */
const UserProvider = ({ children }: { children: React.ReactNode }) => {
  const dispatch = useAppDispatch();
  const token = useAppSelector((state) => state.auth.token);
  const user = useAppSelector((state) => state.user.user);
  const isLoading = useAppSelector((state) => state.user.isLoading);

  useEffect(() => {
    // Fetch user data when token is available and user is not loaded
    if (token && !user && !isLoading) {
      dispatch(fetchCurrentUser());
    }
  }, [token, user, isLoading, dispatch]);

  return <>{children}</>;
};

export default UserProvider;
