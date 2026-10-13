import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 30101,
    strictPort: true,
    // admin 是独立的 npm 工程，Vite 不再把仓库根识别为 workspace 根；
    // 内建模板与跨端 fixture 位于仓库根（examples/、test-fixtures/），需显式放行。
    fs: { allow: [resolve(import.meta.dirname, "..")] },
    proxy: {
      "/api": process.env.VITE_API_PROXY_TARGET ?? "http://127.0.0.1:30100",
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: "./src/test/setup.ts",
    css: true,
  },
});
