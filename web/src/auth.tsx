import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import type { WhoamiInfo } from "../gen/workers/auth/v1/auth_pb.js";
import { authClient, SESSION_STORAGE_KEY, setAuthToken } from "./client";

type Stored = { token: string; whoami: WhoamiInfo };

type AuthState =
  | { status: "loading" }
  | { status: "anonymous" }
  | { status: "authenticated"; token: string; whoami: WhoamiInfo };

type AuthContextValue = {
  state: AuthState;
  setSession: (token: string, whoami: WhoamiInfo) => void;
  refreshWhoami: () => Promise<void>;
  logout: () => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);

function readStored(): Stored | null {
  try {
    const raw = localStorage.getItem(SESSION_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Stored;
    if (!parsed?.token || !parsed?.whoami) return null;
    return parsed;
  } catch {
    return null;
  }
}

function writeStored(value: Stored | null) {
  if (value) localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(value));
  else localStorage.removeItem(SESSION_STORAGE_KEY);
  setAuthToken(value?.token ?? null);
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>({ status: "loading" });
  const tokenRef = useRef<string | null>(null);

  const applySession = useCallback((token: string, whoami: WhoamiInfo) => {
    tokenRef.current = token;
    writeStored({ token, whoami });
    setState({ status: "authenticated", token, whoami });
  }, []);

  useEffect(() => {
    const stored = readStored();
    if (!stored) {
      tokenRef.current = null;
      setAuthToken(null);
      setState({ status: "anonymous" });
      return;
    }
    tokenRef.current = stored.token;
    setAuthToken(stored.token);
    setState({ status: "authenticated", token: stored.token, whoami: stored.whoami });

    let cancelled = false;
    authClient
      .whoami({})
      .then((res) => {
        if (cancelled || !res.whoami) return;
        applySession(stored.token, res.whoami);
      })
      .catch(() => {
        if (cancelled) return;
        tokenRef.current = null;
        writeStored(null);
        setState({ status: "anonymous" });
      });
    return () => {
      cancelled = true;
    };
  }, [applySession]);

  const refreshWhoami = useCallback(async () => {
    const t = tokenRef.current;
    if (!t) return;
    const res = await authClient.whoami({});
    if (!res.whoami) return;
    applySession(t, res.whoami);
  }, [applySession]);

  const logout = useCallback(() => {
    tokenRef.current = null;
    writeStored(null);
    setState({ status: "anonymous" });
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({ state, setSession: applySession, refreshWhoami, logout }),
    [state, applySession, refreshWhoami, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside <AuthProvider>");
  return ctx;
}
