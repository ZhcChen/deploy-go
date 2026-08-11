import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/AppShell";
import { NotFoundPage } from "./NotFoundPage";
import { LoginPage } from "../features/auth/LoginPage";
import { SetupPage } from "../features/auth/SetupPage";
import { AdministratorGuard, SessionGuard } from "./guards";
import { OverviewPage } from "../features/overview/OverviewPage";
import { NodesPage } from "../features/nodes/NodesPage";
import { NodeDetailPage } from "../features/nodes/NodeDetailPage";
import { ApplicationsPage } from "../features/applications/ApplicationsPage";
import { ApplicationTemplatesPage } from "../features/templates/ApplicationTemplatesPage";
import { CreateFromTemplatePage } from "../features/templates/CreateFromTemplatePage";
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
import { AgentReleasesPage } from "../features/agent-releases/AgentReleasesPage";
import { RuntimeLogsPage } from "../features/runtime-logs/RuntimeLogsPage";
import { GitCredentialsPage } from "../features/git-credentials/GitCredentialsPage";
import { ApplicationEnvEditorPage } from "../features/application-envs/ApplicationEnvEditorPage";
import { ExternalApiKeysPage } from "../features/external-keys/ExternalApiKeysPage";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate replace to="/overview" />} />
      <Route path="login" element={<LoginPage />} />
      <Route path="setup" element={<SetupPage />} />
      <Route element={<SessionGuard />}>
        <Route element={<AppShell />}>
          <Route path="overview" element={<OverviewPage />} />
          <Route path="deployments" element={<DeploymentsPage />} />
          <Route path="deployments/new" element={<NewDeploymentPage />} />
          <Route path="deployments/:id" element={<DeploymentDetailPage />} />
          <Route path="apps" element={<ApplicationsPage />} />
          <Route path="templates" element={<ApplicationTemplatesPage />} />
          <Route path="apps/:id" element={<ApplicationDetailPage />} />
          <Route path="apps/:id/targets/:targetId" element={<TargetDetailPage />} />
          <Route path="nodes" element={<NodesPage />} />
          <Route path="nodes/:id" element={<NodeDetailPage />} />
          <Route path="profile" element={<ProfilePage />} />
          <Route element={<AdministratorGuard />}>
            <Route path="apps/:id/config/:envFileId" element={<ApplicationEnvEditorPage />} />
            <Route path="agents" element={<Navigate replace to="/nodes" />} />
            <Route path="agents/:id" element={<Navigate replace to="/nodes" />} />
            <Route path="settings/agent-releases" element={<AgentReleasesPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="templates/new" element={<CreateFromTemplatePage />} />
            <Route path="settings/users" element={<UsersPage />} />
            <Route path="settings/users/:id" element={<UserDetailPage />} />
            <Route path="settings/application-access" element={<ApplicationGrantsPage />} />
            <Route path="settings/git-credentials" element={<GitCredentialsPage />} />
            <Route path="settings/external-api-keys" element={<ExternalApiKeysPage />} />
            <Route path="settings/audit" element={<AuditPage />} />
            <Route path="settings/runtime-logs" element={<RuntimeLogsPage />} />
          </Route>
        </Route>
      </Route>
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}
