import { useNavigate } from 'react-router-dom';

import RotatingTetrahedronCanvas from '../components/RotatingTetrahedronCanvas';

interface WelcomeProps {
  isWeb?: boolean;
}

const Welcome = ({ isWeb }: WelcomeProps) => {
  const navigate = useNavigate();

  return (
    <div className="min-h-full relative flex items-center justify-center">
      <div className="relative z-10 flex w-full max-w-md flex-col items-center gap-7 text-center mx-4 animate-fade-up">
        <div className="h-36 w-36 md:h-44 md:w-44">
          <RotatingTetrahedronCanvas />
        </div>

        <h1 className="text-4xl font-semibold tracking-tight text-white md:text-6xl">OpenHuman</h1>

        <p className="max-w-xl text-sm opacity-70 md:text-base">
          Your AI superhuman for personal and business life.
        </p>

        <button
          className="btn-primary px-8 py-3 text-sm font-medium rounded-xl"
          type="button"
          onClick={() => navigate('/login')}>
          {isWeb ? 'Download OpenHuman' : 'Continue'}
        </button>
      </div>
    </div>
  );
};

export default Welcome;
