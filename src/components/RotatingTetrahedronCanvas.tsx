import { useEffect, useRef } from 'react';
import * as THREE from 'three';

const RotatingTetrahedronCanvas = () => {
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

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
    camera.position.set(0, 0, 4.6);

    const fillMaterial = new THREE.MeshPhongMaterial({
      color: '#7f5af0',
      emissive: '#201040',
      emissiveIntensity: 0.8,
      shininess: 50,
      transparent: true,
      opacity: 0.65,
      flatShading: true,
    });

    const wireMaterial = new THREE.MeshBasicMaterial({
      color: '#a78bfa',
      wireframe: true,
      transparent: true,
      opacity: 0.95,
    });

    const geometry = new THREE.TetrahedronGeometry(1.3, 0);
    const fillMesh = new THREE.Mesh(geometry, fillMaterial);
    const wireMesh = new THREE.Mesh(geometry, wireMaterial);

    scene.add(fillMesh);
    scene.add(wireMesh);

    const ambientLight = new THREE.AmbientLight('#b4b1ff', 0.45);
    const keyLight = new THREE.PointLight('#7f5af0', 1.2, 20);
    keyLight.position.set(2.8, 2.5, 4);

    scene.add(ambientLight);
    scene.add(keyLight);

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
      wireMesh.rotation.copy(fillMesh.rotation);

      renderer.render(scene, camera);
      animationFrame = window.requestAnimationFrame(animate);
    };

    animate();

    return () => {
      window.cancelAnimationFrame(animationFrame);
      observer.disconnect();
      geometry.dispose();
      fillMaterial.dispose();
      wireMaterial.dispose();
      renderer.dispose();
    };
  }, []);

  return <canvas ref={canvasRef} className="h-full w-full" aria-label="Rotating tetrahedron animation" />;
};

export default RotatingTetrahedronCanvas;
