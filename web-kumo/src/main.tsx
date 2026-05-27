import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { ToastProvider, TooltipProvider } from "@cloudflare/kumo";
import { App } from "./App";
import { AuthProvider } from "./auth";
import { applyTheme, readInitialTheme } from "./theme";
import "./styles.css";

// Apply the saved theme BEFORE React mounts so every page (not just
// /preview where ThemeToggle lives) reflects the user's choice.
applyTheme(readInitialTheme());

const root = document.getElementById("root");
if (!root) throw new Error("missing #root");

createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        {/* Kumo providers — ToastProvider holds the toast queue;
         * TooltipProvider gates the shared open-delay state for all
         * Tooltips on the page. Overlay components (Dialog, Popover,
         * DropdownMenu, CommandPalette) portal to document.body by
         * default; only wrap in KumoPortalProvider if you need a
         * custom container (Shadow DOM, etc.). */}
        <TooltipProvider>
          <ToastProvider>
            <App />
          </ToastProvider>
        </TooltipProvider>
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
);
