import type { NextConfig } from 'next';
import path from 'path';
import { fileURLToPath } from 'node:url';

// Pin Turbopack root to this app so a parent lockfile (e.g. ~/yarn.lock) is not chosen as the workspace root.
const landerRoot = path.dirname(fileURLToPath(import.meta.url));

// Enables static export so the landing site can be hosted on GitHub Pages.
// `NEXT_PUBLIC_BASE_PATH` is used so project pages can be deployed under `/${repo}`.
const basePath = process.env.NEXT_PUBLIC_BASE_PATH;
const normalizedBasePath = basePath && basePath.trim().length > 0 ? basePath : undefined;

const nextConfig: NextConfig = {
  output: 'export',
  trailingSlash: true,
  basePath: normalizedBasePath,
  assetPrefix: normalizedBasePath,
  turbopack: {
    root: landerRoot,
  },
};

export default nextConfig;
