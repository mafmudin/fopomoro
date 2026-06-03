import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config";

// Inherit the Svelte plugin from vite.config so tests that import
// `.svelte` components transform correctly.
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      include: ["src/**/*.test.ts"],
    },
  })
);
