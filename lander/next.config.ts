import type { NextConfig } from "next";

// Enables static export so the landing site can be hosted on GitHub Pages.
// `NEXT_PUBLIC_BASE_PATH` is used so project pages can be deployed under `/${repo}`.
const basePath = process.env.NEXT_PUBLIC_BASE_PATH;
const normalizedBasePath = basePath && basePath.trim().length > 0 ? basePath : undefined;

const nextConfig: NextConfig = {
  output: "export",
  trailingSlash: true,
  basePath: normalizedBasePath,
  assetPrefix: normalizedBasePath,
};

export default nextConfig;
