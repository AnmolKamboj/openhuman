'use client';

import { Canvas } from '@react-three/fiber';
import { PerspectiveCamera } from '@react-three/drei';
import AnimatedLogo from './AnimatedLogo';

export default function LogoCanvas() {
  return (
    <div className="w-[150px] h-[150px] mx-auto mb-4">
      <Canvas>
        <PerspectiveCamera makeDefault position={[0, 0, 5]} fov={50} />
        <ambientLight intensity={0.5} />
        <pointLight position={[10, 10, 10]} intensity={1} />
        <pointLight position={[-10, -10, -10]} intensity={0.5} />
        <directionalLight position={[0, 5, 5]} intensity={0.8} />
        <AnimatedLogo />
      </Canvas>
    </div>
  );
}
