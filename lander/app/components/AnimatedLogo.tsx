'use client';

import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Group, OctahedronGeometry } from 'three';
import { useMousePosition } from '../hooks/useMousePosition';
import { Edges } from '@react-three/drei';

export default function AnimatedLogo() {
  const groupRef = useRef<Group>(null);
  const mousePosition = useMousePosition();

  useFrame(() => {
    if (groupRef.current) {
      // Subtle rotation based on mouse position
      groupRef.current.rotation.y = mousePosition.x * 0.3 - 0.15;
      groupRef.current.rotation.x = -mousePosition.y * 0.3 + 0.15;

      // Slow continuous rotation
      groupRef.current.rotation.z += 0.002;
    }
  });

  return (
    <group ref={groupRef} position={[0, 0, 0]}>
      {/* Main octahedron */}
      <mesh>
        <octahedronGeometry args={[1.5, 0]} />
        <meshStandardMaterial
          color="#ffffff"
          metalness={0.8}
          roughness={0.2}
          emissive="#ffffff"
          emissiveIntensity={0.1}
        />
        <Edges color="#666666" />
      </mesh>

      {/* Inner layer for depth */}
      <mesh scale={0.7}>
        <octahedronGeometry args={[1.5, 0]} />
        <meshStandardMaterial
          color="#e5e5e5"
          metalness={0.7}
          roughness={0.3}
          transparent
          opacity={0.6}
        />
      </mesh>
    </group>
  );
}
