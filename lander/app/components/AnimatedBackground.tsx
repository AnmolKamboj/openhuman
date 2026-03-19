'use client';

import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Mesh, ShaderMaterial, Vector2 } from 'three';

interface AnimatedBackgroundProps {
  mousePosition: { x: number; y: number };
}

export default function AnimatedBackground({ mousePosition }: AnimatedBackgroundProps) {
  const meshRef = useRef<Mesh>(null);
  const uniforms = useMemo(
    () => ({
      uMouse: { value: new Vector2(0.5, 0.5) },
      uTime: { value: 0 },
    }),
    []
  );

  useFrame((state) => {
    if (meshRef.current) {
      const material = meshRef.current.material as ShaderMaterial;

      // Update uniforms
      if (material.uniforms) {
        material.uniforms.uMouse.value.set(mousePosition.x, mousePosition.y);
        material.uniforms.uTime.value = state.clock.getElapsedTime();
      }

      // Subtle rotation based on mouse position
      meshRef.current.rotation.x = mousePosition.y * 0.1;
      meshRef.current.rotation.y = mousePosition.x * 0.1;
    }
  });

  return (
    <mesh ref={meshRef} position={[0, 0, -5]}>
      <planeGeometry args={[50, 50, 64, 64]} />
        <shaderMaterial
        vertexShader={`
          varying vec2 vUv;
          varying vec3 vPosition;

          void main() {
            vUv = uv;
            vPosition = position;
            gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
          }
        `}
        fragmentShader={`
          uniform vec2 uMouse;
          uniform float uTime;
          varying vec2 vUv;
          varying vec3 vPosition;

          void main() {
            vec2 uv = vUv;

            // Create subtle gradient based on mouse position
            vec2 mouseInfluence = (uMouse - 0.5) * 2.0;
            float dist = distance(uv, vec2(0.5) + mouseInfluence * 0.3);

            // Create subtle wave pattern
            float wave = sin(dist * 10.0 - uTime * 0.5) * 0.5 + 0.5;

            // Dark base with subtle variations
            vec3 color1 = vec3(0.0, 0.0, 0.0);
            vec3 color2 = vec3(0.02, 0.02, 0.03);
            vec3 color3 = vec3(0.01, 0.01, 0.02);

            // Mix colors based on distance and wave
            vec3 color = mix(color1, color2, wave * 0.3);
            color = mix(color, color3, dist * 0.5);

            // Add subtle glow near mouse position
            float glow = 1.0 / (1.0 + dist * 15.0);
            color += vec3(0.01, 0.01, 0.015) * glow * 0.3;

            gl_FragColor = vec4(color, 1.0);
          }
        `}
        uniforms={uniforms}
      />
    </mesh>
  );
}
