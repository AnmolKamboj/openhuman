'use client';

import { Canvas } from '@react-three/fiber';
import { useMousePosition } from '../hooks/useMousePosition';
import AnimatedBackground from './AnimatedBackground';

export default function BackgroundCanvas() {
  const mousePosition = useMousePosition();

  return (
    <div className="fixed inset-0 -z-10">
      <Canvas
        camera={{ position: [0, 0, 5], fov: 75 }}
        gl={{ alpha: false, antialias: true }}
      >
        <color attach="background" args={['#000000']} />
        <AnimatedBackground mousePosition={mousePosition} />
      </Canvas>
    </div>
  );
}
