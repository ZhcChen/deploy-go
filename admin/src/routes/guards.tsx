import { Navigate, Outlet, useLocation } from "react-router-dom";
import { PageState } from "../components/PageState";
import { useAuth } from "../features/auth/AuthContext";
import { ForbiddenPage } from "../features/errors/ForbiddenPage";
import { ServiceUnavailablePage } from "../features/errors/ServiceUnavailablePage";

export function SessionGuard() {
  const auth = useAuth();
  const location = useLocation();
  if (auth.status === "booting") return <main className="public-page"><PageState kind="loading" /></main>;
  if (auth.status === "unavailable") return <ServiceUnavailablePage />;
  if (auth.status === "setup_required" || auth.status === "setup_disabled") return <Navigate replace to="/setup" />;
  if (auth.status === "anonymous") {
    return <Navigate replace to="/login" state={{ from: `${location.pathname}${location.search}` }} />;
  }
  return <Outlet />;
}

export function AdministratorGuard() {
  const { user } = useAuth();
  return user?.identity === "administrator" ? <Outlet /> : <ForbiddenPage />;
}
