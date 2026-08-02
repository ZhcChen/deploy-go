import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/AppShell";
import { NotFoundPage } from "./NotFoundPage";
import { PlaceholderPage } from "./PlaceholderPage";
import { LoginPage } from "../features/auth/LoginPage";
import { SetupPage } from "../features/auth/SetupPage";
import { AdministratorGuard, SessionGuard } from "./guards";
import { CredentialsPage } from "../features/credentials/CredentialsPage";
import { CredentialDetailPage } from "../features/credentials/CredentialDetailPage";
import { NodesPage } from "../features/nodes/NodesPage";
import { NodeDetailPage } from "../features/nodes/NodeDetailPage";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate replace to="/overview" />} />
      <Route path="login" element={<LoginPage />} />
      <Route path="setup" element={<SetupPage />} />
      <Route element={<SessionGuard />}>
        <Route element={<AppShell />}>
          <Route path="overview" element={<PlaceholderPage label="运行概览" />} />
          <Route path="deployments/*" element={<PlaceholderPage label="部署记录" />} />
          <Route path="apps/*" element={<PlaceholderPage label="应用管理" />} />
          <Route path="nodes" element={<NodesPage />} />
          <Route path="nodes/:id" element={<NodeDetailPage />} />
          <Route element={<AdministratorGuard />}>
            <Route path="settings" element={<PlaceholderPage label="系统设置" />} />
            <Route path="settings/users/*" element={<PlaceholderPage label="用户管理" />} />
            <Route path="settings/credentials" element={<CredentialsPage />} />
            <Route path="settings/credentials/:id" element={<CredentialDetailPage />} />
            <Route path="settings/audit/*" element={<PlaceholderPage label="审计记录" />} />
          </Route>
        </Route>
      </Route>
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}
