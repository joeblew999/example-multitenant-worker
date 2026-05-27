import { Sidebar } from "@cloudflare/kumo";
import { useNavigate, useLocation, Link } from "react-router-dom";
import {
  HouseIcon,
  UsersIcon,
  CreditCardIcon,
  EnvelopeSimpleIcon,
  SignOutIcon,
} from "@phosphor-icons/react";
import { useAuth } from "../auth";

interface NavItem {
  to: string;
  label: string;
  icon: typeof HouseIcon;
}

const NAV: NavItem[] = [
  { to: "/", label: "Dashboard", icon: HouseIcon },
  { to: "/members", label: "Members", icon: UsersIcon },
  { to: "/billing", label: "Billing", icon: CreditCardIcon },
  { to: "/invitations", label: "Invitations", icon: EnvelopeSimpleIcon },
];

export function AppShell({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { logout } = useAuth();

  return (
    <Sidebar.Provider defaultOpen>
      <div className="flex h-full w-full bg-kumo-base">
        <Sidebar>
          <Sidebar.Content>
            <Sidebar.Group>
              <Sidebar.GroupLabel>
                <Link to="/" className="flex items-center gap-2">
                  <span className="size-3 shrink-0 rounded bg-kumo-brand" />
                  <span className="font-semibold text-kumo-strong">
                    Multitenant
                  </span>
                </Link>
              </Sidebar.GroupLabel>
            </Sidebar.Group>

            <Sidebar.Group>
              <Sidebar.GroupLabel>Workspace</Sidebar.GroupLabel>
              <Sidebar.Menu>
                {NAV.map((item) => (
                  <Sidebar.MenuButton
                    key={item.to}
                    icon={item.icon}
                    active={pathname === item.to}
                    onClick={() => navigate(item.to)}
                  >
                    {item.label}
                  </Sidebar.MenuButton>
                ))}
              </Sidebar.Menu>
            </Sidebar.Group>
          </Sidebar.Content>

          <Sidebar.Footer>
            <Sidebar.Menu>
              <Sidebar.MenuButton icon={SignOutIcon} onClick={logout}>
                Sign out
              </Sidebar.MenuButton>
            </Sidebar.Menu>
          </Sidebar.Footer>
        </Sidebar>

        <main className="flex-1 overflow-y-auto p-6">
          {children}
        </main>
      </div>
    </Sidebar.Provider>
  );
}
