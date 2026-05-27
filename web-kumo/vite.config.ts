import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import basicSsl from "@vitejs/plugin-basic-ssl";

// HTTPS locally. Wrangler should also run with HTTPS:
//   wrangler dev --local-protocol=https
// The proxy below uses https://127.0.0.1:8787 to match. Self-signed certs
// on both ends — accept them once in the browser.
const WORKER_ORIGIN = "https://127.0.0.1:8787";

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
