import { Button, Sidebar, useSidebar } from "@cloudflare/kumo";
import { useNavigate, useLocation, Link } from "react-router-dom";
import { ListIcon, SignOutIcon } from "@phosphor-icons/react";
import { useAuth } from "../auth";
import { devAccountsEnabled } from "../dev-flags";
import { visibleNavItems } from "../nav";

function MenuButton() {
  const { setOpenMobile, isMobile } = useSidebar();
  if (!isMobile) return null;
  return (
    <Button
      variant="secondary"
      size="sm"
      onClick={() => setOpenMobile(true)}
      aria-label="Open menu"
      className="md:hidden mb-4 shrink-0"
    >
      <ListIcon size={16} />
      Menu
    </Button>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { state, logout } = useAuth();

  // Pull whoami if we have it (will be null on /preview which is
  // public). visibleNavItems handles the null case — returns only
  // items with no visibility predicate.
  const whoami = state.status === "authenticated" ? state.whoami : null;
  const navItems = visibleNavItems(whoami);

  // On dev/demo builds (DevAccounts visible), bounce post-logout to
  // /preview so the next sign-in is one tap away. On real production
  // (test accounts hidden), fall through to the route guard's default
  // anonymous redirect → /login.
  const handleSignOut = () => {
    logout();
    if (devAccountsEnabled()) navigate("/preview", { replace: true });
  };

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
                {navItems.map((item) => (
                  <Sidebar.MenuButton
                    key={item.path}
                    icon={item.nav!.icon}
                    active={pathname === item.path}
                    onClick={() => navigate(item.path)}
                  >
                    {item.nav!.label}
                  </Sidebar.MenuButton>
                ))}
              </Sidebar.Menu>
            </Sidebar.Group>
          </Sidebar.Content>

          <Sidebar.Footer>
            <Sidebar.Menu>
              <Sidebar.MenuButton icon={SignOutIcon} onClick={handleSignOut}>
                Sign out
              </Sidebar.MenuButton>
            </Sidebar.Menu>
          </Sidebar.Footer>
        </Sidebar>

        <main className="flex-1 overflow-y-auto p-6">
          <MenuButton />
          {children}
        </main>
      </div>
    </Sidebar.Provider>
  );
}
