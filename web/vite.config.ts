import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// During `vite dev`, proxy ConnectRPC + non-RPC HTTP routes to a locally
// running `wrangler dev` so the SPA can talk to the worker exactly as it
// will in production (where the worker serves both the assets and the API).
const WORKER_ORIGIN = "http://127.0.0.1:8787";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "^/workers\\.": { target: WORKER_ORIGIN, changeOrigin: false },
      "^/healthz$": { target: WORKER_ORIGIN, changeOrigin: false },
      "^/oauth/": { target: WORKER_ORIGIN, changeOrigin: false },
      "^/verify-email": { target: WORKER_ORIGIN, changeOrigin: false },
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
