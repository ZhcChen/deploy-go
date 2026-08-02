import { ChevronRight, LogOut } from "lucide-react";
import { useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { metadataFor, primaryRoutes, settingsRoutes } from "../routes/routeMetadata";
import { useAuth } from "../features/auth/AuthContext";

function navClass({ isActive }: { isActive: boolean }) {
  return `nav-link${isActive ? " is-active" : ""}`;
}

export function AppShell() {
  const { pathname } = useLocation();
  const auth = useAuth();
  const [loggingOut, setLoggingOut] = useState(false);
  const [logoutError, setLogoutError] = useState(false);
  const route = metadataFor(pathname);
  const inSettings = pathname === "/settings" || pathname.startsWith("/settings/");
  const isAdministrator = auth.user?.identity === "administrator";
  const visibleRoutes = isAdministrator
    ? primaryRoutes
    : primaryRoutes.filter(({ path }) => path !== "/settings");

  async function logout() {
    if (loggingOut) return;
    setLoggingOut(true);
    setLogoutError(false);
    try {
      await auth.logout();
    } catch {
      setLogoutError(true);
    } finally {
      setLoggingOut(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <NavLink className="brand" to="/overview" aria-label="Deploy Go 概览">
          <span className="brand-mark">DG</span>
          <strong>Deploy Go</strong>
        </NavLink>
        <nav className="primary-nav" aria-label="主导航">
          {visibleRoutes.map(({ path, label, icon: Icon }) => (
            <div key={path}>
              <NavLink className={navClass} to={path} end={path === "/settings"}>
                <Icon aria-hidden="true" />
                <span>{label}</span>
              </NavLink>
              {path === "/settings" && inSettings ? (
                <nav className="settings-nav" aria-label="设置导航">
                  {settingsRoutes.map(({ path: childPath, label, icon: Icon }) => (
                    <NavLink
                      className={navClass}
                      key={childPath}
                      to={childPath}
                      end={childPath === "/settings"}
                    >
                      <Icon aria-hidden="true" />
                      <span>{label}</span>
                    </NavLink>
                  ))}
                </nav>
              ) : null}
            </div>
          ))}
        </nav>
        <div className="sidebar-account">
          <span className="avatar" aria-hidden="true">{auth.user?.displayName.slice(0, 1) ?? "U"}</span>
          <span><strong>{auth.user?.displayName}</strong><small>{isAdministrator ? "管理员" : "普通用户"}</small></span>
          <button className="icon-button icon-button--dark" type="button" aria-label="退出登录" disabled={loggingOut} onClick={() => void logout()}>
            <LogOut aria-hidden="true" />
          </button>
        </div>
        {logoutError ? <p className="sidebar-error" role="alert">退出失败，请重试</p> : null}
      </aside>
      <main className="main-column">
        <header className="page-header">
          <div>
            <div className="breadcrumb"><span>Deploy Go</span><ChevronRight aria-hidden="true" /><span>{route?.title ?? "页面"}</span></div>
            <h1>{route?.title ?? "页面"}</h1>
          </div>
        </header>
        <div className="page-content">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
