'use client';

import { useEffect, useRef } from 'react';
import * as THREE from 'three';

export default function RotatingTetrahedronCanvas() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      alpha: true,
      powerPreference: 'high-performance',
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.15;

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
    camera.position.set(0, 0, 4.6);

    // Subdivided tet + smooth normals ≈ Oblivion-style blunted / softened silhouette (not razor facets).
    const geometry = new THREE.TetrahedronGeometry(1.35, 2);
    geometry.computeVertexNormals();

    const fillMaterial = new THREE.MeshPhysicalMaterial({
      color: '#5b8fd9',
      emissive: '#0d1f35',
      emissiveIntensity: 0.45,
      metalness: 0.42,
      roughness: 0.18,
      clearcoat: 0.92,
      clearcoatRoughness: 0.12,
      transparent: true,
      opacity: 0.9,
      flatShading: false,
    });

    const fillMesh = new THREE.Mesh(geometry, fillMaterial);
    scene.add(fillMesh);

    const ambientLight = new THREE.AmbientLight('#cbd5e1', 0.35);
    const keyLight = new THREE.PointLight('#7ab4ff', 1.35, 24);
    keyLight.position.set(2.8, 2.5, 4);
    const rimLight = new THREE.PointLight('#4a83dd', 0.85, 18);
    rimLight.position.set(-3.2, -1.2, 3.5);
    const fillLight = new THREE.DirectionalLight('#e8f0ff', 0.55);
    fillLight.position.set(-1.5, 2, 6);

    scene.add(ambientLight);
    scene.add(keyLight);
    scene.add(rimLight);
    scene.add(fillLight);

    let animationFrame = 0;

    const resize = () => {
      const parent = canvas.parentElement;
      if (!parent) return;

      const { width, height } = parent.getBoundingClientRect();
      if (!width || !height) return;

      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };

    const observer = new ResizeObserver(resize);
    if (canvas.parentElement) observer.observe(canvas.parentElement);
    resize();

    const animate = () => {
      fillMesh.rotation.x += 0.006;
      fillMesh.rotation.y += 0.009;

      renderer.render(scene, camera);
      animationFrame = window.requestAnimationFrame(animate);
    };

    animate();

    return () => {
      window.cancelAnimationFrame(animationFrame);
      observer.disconnect();
      geometry.dispose();
      fillMaterial.dispose();
      renderer.dispose();
    };
  }, []);

  return (
    <canvas ref={canvasRef} className="h-full w-full block" aria-label="Rotating tetrahedron" />
  );
}
