/// <reference types="node" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import basicSsl from "@vitejs/plugin-basic-ssl";

// HTTPS locally. Wrangler should also run with HTTPS:
//   wrangler dev --local-protocol=https
// The proxy below uses https://127.0.0.1:8787 to match. Self-signed certs
// on both ends — accept them once in the browser.
const WORKER_ORIGIN = "https://127.0.0.1:8787";

// Default scenario when the env var isn't set. Override via shell env
// (`VITE_SEED_SCENARIO=remysport pnpm dev`). Set here in config rather
// than a .env file because globally-gitignored .env files don't survive
// fresh clones — and we need `%VITE_SEED_SCENARIO%` in index.html to
// resolve at build time.
process.env.VITE_SEED_SCENARIO ??= "editorial";

export default defineConfig({
  plugins: [react(), tailwindcss(), basicSsl()],
  server: {
    port: 5173,
    proxy: {
      "^/workers\\.": {
        target: WORKER_ORIGIN,
        changeOrigin: false,
        secure: false, // accept wrangler's self-signed cert
      },
      "^/healthz$": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
      "^/oauth/": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
      "^/verify-email": { target: WORKER_ORIGIN, changeOrigin: false, secure: false },
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
