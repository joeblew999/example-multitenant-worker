/// <reference types="node" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// HTTP-only dev variant. The default vite.config.ts uses basicSsl for
// production-parity (Secure cookies). This config exists so headless
// browser tools (Claude preview, Playwright in CI) that reject
// self-signed certs can still drive the UI. Browser ↔ Vite is HTTP;
// Vite ↔ Wrangler proxy is HTTPS (server-side, cert ignored).
const WORKER_ORIGIN = "https://127.0.0.1:8787";

// Default scenario; override via shell env. Same pattern as vite.config.ts.
process.env.VITE_SEED_SCENARIO ??= "editorial";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5175,
    proxy: {
      "^/workers\\.": {
        target: WORKER_ORIGIN,
        changeOrigin: false,
        secure: false,
      },
      "^/healthz$": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
      "^/oauth/": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
      "^/verify-email": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
    },
  },
});
