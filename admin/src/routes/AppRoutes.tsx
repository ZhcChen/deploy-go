import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/AppShell";
import { NotFoundPage } from "./NotFoundPage";
import { PlaceholderPage } from "./PlaceholderPage";
import { LoginPage } from "../features/auth/LoginPage";
import { SetupPage } from "../features/auth/SetupPage";
import { AdministratorGuard, SessionGuard } from "./guards";
import { NodesPage } from "../features/nodes/NodesPage";
import { NodeDetailPage } from "../features/nodes/NodeDetailPage";
import { ApplicationsPage } from "../features/applications/ApplicationsPage";
import { ApplicationDetailPage } from "../features/applications/ApplicationDetailPage";
import { TargetDetailPage } from "../features/targets/TargetDetailPage";
import { ApplicationGrantsPage } from "../features/grants/ApplicationGrantsPage";
import { UsersPage } from "../features/users/UsersPage";
import { UserDetailPage } from "../features/users/UserDetailPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { AuditPage } from "../features/audit/AuditPage";
import { ProfilePage } from "../features/profile/ProfilePage";
import { DeploymentsPage } from "../features/deployments/DeploymentsPage";
import { NewDeploymentPage } from "../features/deployments/NewDeploymentPage";
import { DeploymentDetailPage } from "../features/deployments/DeploymentDetailPage";
import { AgentsPage } from "../features/agents/AgentsPage";
import { AgentDetailPage } from "../features/agents/AgentDetailPage";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate replace to="/overview" />} />
      <Route path="login" element={<LoginPage />} />
      <Route path="setup" element={<SetupPage />} />
      <Route element={<SessionGuard />}>
        <Route element={<AppShell />}>
          <Route path="overview" element={<PlaceholderPage label="运行概览" />} />
          <Route path="deployments" element={<DeploymentsPage />} />
          <Route path="deployments/new" element={<NewDeploymentPage />} />
          <Route path="deployments/:id" element={<DeploymentDetailPage />} />
          <Route path="apps" element={<ApplicationsPage />} />
          <Route path="apps/:id" element={<ApplicationDetailPage />} />
          <Route path="apps/:id/targets/:targetId" element={<TargetDetailPage />} />
          <Route path="nodes" element={<NodesPage />} />
          <Route path="nodes/:id" element={<NodeDetailPage />} />
          <Route path="profile" element={<ProfilePage />} />
          <Route element={<AdministratorGuard />}>
            <Route path="agents" element={<AgentsPage />} />
            <Route path="agents/:id" element={<AgentDetailPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="settings/users" element={<UsersPage />} />
            <Route path="settings/users/:id" element={<UserDetailPage />} />
            <Route path="settings/application-access" element={<ApplicationGrantsPage />} />
            <Route path="settings/audit" element={<AuditPage />} />
          </Route>
        </Route>
      </Route>
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}
