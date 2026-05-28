/**
 * Single source of truth for AppShell routes and the sidebar nav.
 *
 * Each entry has a path + page component, plus optional `nav` metadata
 * that says "this route gets a sidebar entry, here's its label/icon,
 * and here's the predicate that decides whether to show it to this
 * user."
 *
 * Today the predicate is synchronous over the WhoamiInfo — fine
 * for role-based visibility (e.g. "only owners see Billing"). When the
 * Cedar middleware lands, the predicate body can call into a cached
 * policy result on whoami (e.g. whoami.permissions?.includes("billing:read"))
 * without changing the interface — App.tsx and AppShell.tsx don't care
 * how the boolean comes back.
 *
 * Routes WITHOUT a `nav` block exist but don't appear in the sidebar
 * (use this for /preview, internal debugging routes, etc.).
 */

import type { ComponentType } from "react";
import {
  CreditCardIcon,
  EnvelopeSimpleIcon,
  HouseIcon,
  type Icon,
  UsersIcon,
} from "@phosphor-icons/react";
import type { WhoamiInfo } from "../gen/workers/auth/v1/auth_pb.js";
import { Billing } from "./pages/Billing";
import { Dashboard } from "./pages/Dashboard";
import { Invitations } from "./pages/Invitations";
import { Members } from "./pages/Members";
import { Preview } from "./pages/Preview";

export interface AppRoute {
  /** URL path (passed to react-router's <Route path>). */
  path: string;
  /** Page component. Wrapped in <AppShell> by the router consumer. */
  element: ComponentType;
  /** When defined, route gets a sidebar entry. */
  nav?: {
    label: string;
    icon: Icon;
    /**
     * Return false to hide from the sidebar. Default: always visible.
     * Today: role-based. Tomorrow: Cedar policy check over a cached
     * permissions list on whoami.
     */
    visible?: (whoami: WhoamiInfo) => boolean;
  };
  /** True if the route requires an authenticated session. */
  requireAuth: boolean;
}

export const APP_ROUTES: AppRoute[] = [
  {
    path: "/",
    element: Dashboard,
    requireAuth: true,
    nav: { label: "Dashboard", icon: HouseIcon },
  },
  {
    path: "/members",
    element: Members,
    requireAuth: true,
    nav: { label: "Members", icon: UsersIcon },
  },
  {
    path: "/billing",
    element: Billing,
    requireAuth: true,
    nav: { label: "Billing", icon: CreditCardIcon },
    // Future: { ..., visible: (whoami) => canViewBilling(whoami) }
  },
  {
    path: "/invitations",
    element: Invitations,
    requireAuth: true,
    nav: { label: "Invitations", icon: EnvelopeSimpleIcon },
  },
  {
    // Dev/QA showcase. Public (no auth) but lives inside AppShell so
    // the sidebar is available for navigation back. No nav entry —
    // /preview is for developers, not surfaced to users.
    path: "/preview",
    element: Preview,
    requireAuth: false,
  },
];

/** AppShell sidebar items visible to the given session. */
export function visibleNavItems(whoami: WhoamiInfo | null): AppRoute[] {
  return APP_ROUTES.filter((r) => {
    if (!r.nav) return false;
    if (!r.nav.visible) return true;
    if (!whoami) return false;
    return r.nav.visible(whoami);
  });
}
