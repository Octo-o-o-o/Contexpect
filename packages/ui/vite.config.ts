import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": {
        // The daemon's default listen address; CTXPECT_DAEMON_URL overrides
        // it for a dev setup that points at a non-default daemon.
        target: process.env.CTXPECT_DAEMON_URL ?? "http://127.0.0.1:7420",
        changeOrigin: false,
      },
    },
  },
  build: {
    outDir: "dist",
    sourcemap: true,
  },
});
