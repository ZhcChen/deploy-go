import {
  createContext,
  type PropsWithChildren,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { LoginRequest } from "../../api/generated/models/LoginRequest";
import type { SetupRequest } from "../../api/generated/models/SetupRequest";
import type { UserIdentity } from "../../api/generated/models/UserIdentity";
import { authApi, normalizeApiError, onUnauthorized, type ApiError } from "../../api/http-client";

export type AuthStatus = "booting" | "setup_required" | "setup_disabled" | "anonymous" | "authenticated" | "unavailable";

export interface AuthSnapshot {
  status: AuthStatus;
  user: UserIdentity | null;
  csrfToken: string | null;
  error?: ApiError;
  sessionExpired?: boolean;
}

interface AuthContextValue extends AuthSnapshot {
  retry(): Promise<void>;
  login(request: LoginRequest): Promise<void>;
  setup(token: string, request: SetupRequest): Promise<void>;
  logout(): Promise<void>;
  applyUser(user: UserIdentity): void;
}

const AuthContext = createContext<AuthContextValue | null>(null);
const emptySnapshot: AuthSnapshot = { status: "booting", user: null, csrfToken: null };

export function AuthProvider({ children, initialSnapshot }: PropsWithChildren<{ initialSnapshot?: AuthSnapshot }>) {
  const [snapshot, setSnapshot] = useState<AuthSnapshot>(initialSnapshot ?? emptySnapshot);
  const pendingLogin = useRef<Promise<void> | null>(null);
  const queryClient = useQueryClient();
  const clearIdentityCache = useCallback(() => queryClient.clear(), [queryClient]);

  const bootstrap = useCallback(async () => {
    try {
      const setupStatus = await authApi.authSetupStatus();
      if (setupStatus.setupRequired) {
        setSnapshot({
          status: setupStatus.setupEnabled ? "setup_required" : "setup_disabled",
          user: null,
          csrfToken: null,
        });
        return;
      }
      const user = await authApi.authMe();
      const csrf = await authApi.authRefreshCsrf({
        origin: window.location.origin,
        secFetchSite: "same-origin",
        secFetchMode: "cors",
      });
      clearIdentityCache();
      setSnapshot({ status: "authenticated", user, csrfToken: csrf.csrfToken });
    } catch (cause) {
      const error = await normalizeApiError(cause);
      if (error.status === 401) {
        setSnapshot({ status: "anonymous", user: null, csrfToken: null });
      } else {
        setSnapshot({ status: "unavailable", user: null, csrfToken: null, error });
      }
    }
  }, [clearIdentityCache]);

  const retry = useCallback(async () => {
    setSnapshot(emptySnapshot);
    await bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    if (initialSnapshot) return;
    const timeout = window.setTimeout(() => void bootstrap(), 0);
    return () => window.clearTimeout(timeout);
  }, [bootstrap, initialSnapshot]);

  useEffect(
    () =>
      onUnauthorized(() => {
        clearIdentityCache();
        setSnapshot((current) =>
          current.status === "authenticated"
            ? { status: "anonymous", user: null, csrfToken: null, sessionExpired: true }
            : current,
        );
      }),
    [clearIdentityCache],
  );

  const login = useCallback(async (request: LoginRequest) => {
    if (pendingLogin.current) return pendingLogin.current;
    const operation = (async () => {
      try {
        const session = await authApi.authLogin({ origin: window.location.origin, loginRequest: request });
        clearIdentityCache();
        setSnapshot({ status: "authenticated", user: session.user, csrfToken: session.csrfToken });
      } catch (cause) {
        throw await normalizeApiError(cause);
      } finally {
        pendingLogin.current = null;
      }
    })();
    pendingLogin.current = operation;
    return operation;
  }, [clearIdentityCache]);

  const setup = useCallback(async (token: string, request: SetupRequest) => {
    try {
      await authApi.authSetup({ xSetupToken: token, origin: window.location.origin, setupRequest: request });
      clearIdentityCache();
      setSnapshot({ status: "anonymous", user: null, csrfToken: null });
    } catch (cause) {
      throw await normalizeApiError(cause);
    }
  }, [clearIdentityCache]);

  const logout = useCallback(async () => {
    if (!snapshot.csrfToken) throw new Error("缺少 CSRF token");
    try {
      await authApi.authLogout({ xCSRFToken: snapshot.csrfToken });
      clearIdentityCache();
      setSnapshot({ status: "anonymous", user: null, csrfToken: null });
    } catch (cause) {
      const error = await normalizeApiError(cause);
      if (error.status === 401) {
        clearIdentityCache();
        setSnapshot({ status: "anonymous", user: null, csrfToken: null, sessionExpired: true });
        return;
      }
      throw error;
    }
  }, [clearIdentityCache, snapshot.csrfToken]);

  const applyUser = useCallback((user: UserIdentity) => {
    setSnapshot((current) => ({ ...current, user }));
  }, []);

  const value = useMemo(
    () => ({ ...snapshot, retry, login, setup, logout, applyUser }),
    [snapshot, retry, login, setup, logout, applyUser],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) throw new Error("useAuth 必须在 AuthProvider 内使用");
  return context;
}
