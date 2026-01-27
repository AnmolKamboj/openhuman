import { useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAppDispatch } from '../store/hooks';
import { setToken } from '../store/authSlice';

const Login = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const dispatch = useAppDispatch();

  // Handle JWT token from URL query parameter (backend redirect)
  useEffect(() => {
    const token = searchParams.get('token');

    if (token) {

      // Store the JWT token from the backend
      dispatch(setToken(token));

      // Clear the token from URL for security
      const newSearchParams = new URLSearchParams(searchParams);
      newSearchParams.delete('token');
      const newSearch = newSearchParams.toString();
      const newUrl = newSearch ? `${window.location.pathname}?${newSearch}` : window.location.pathname;
      window.history.replaceState({}, '', newUrl);

      // Navigate to onboarding after successful login
      setTimeout(() => {
        navigate('/onboarding/');
      }, 100);
    }
  }, [searchParams, dispatch, navigate]);



  return (
    <div className="min-h-screen relative flex items-center justify-center">
      <div className="relative z-10 max-w-md w-full mx-4 text-center">
        <div className="glass rounded-3xl p-8 shadow-large animate-fade-up">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-white mx-auto mb-4"></div>
          <p className="opacity-70">Completing login...</p>
        </div>
      </div>
    </div>
  );
};

export default Login;
