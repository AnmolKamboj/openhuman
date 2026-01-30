import { defineConfig } from "vitest/config";
import { nodePolyfills } from "vite-plugin-node-polyfills";

export default defineConfig({
  plugins: [
    nodePolyfills({
      include: ["buffer", "process", "util", "os", "crypto", "stream"],
      globals: {
        Buffer: true,
        process: true,
        global: true,
      },
    }),
  ],
  resolve: {
    alias: {
      buffer: "buffer",
      process: "process/browser",
      util: "util",
      os: "os-browserify/browser",
    },
  },
  test: {
    globals: true,
    environment: "node",
    mockReset: true,
    restoreMocks: true,
    setupFiles: ["src/__tests__/setup.ts"],
    include: ["src/**/*.test.ts"],
    hookTimeout: 30000,
    testTimeout: 30000,
    coverage: {
      provider: "v8",
      include: [
        "src/lib/mcp/**",
        "src/store/telegram/**",
        "src/services/updateManager.ts",
        "src/services/messageLoader.ts",
      ],
      reporter: ["text", "html"],
    },
  },
});
