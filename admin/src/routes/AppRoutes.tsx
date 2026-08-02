import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "../components/AppShell";
import { NotFoundPage } from "./NotFoundPage";
import { PlaceholderPage } from "./PlaceholderPage";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate replace to="/overview" />} />
      <Route element={<AppShell />}>
        <Route path="overview" element={<PlaceholderPage label="运行概览" />} />
        <Route path="deployments/*" element={<PlaceholderPage label="部署记录" />} />
        <Route path="apps/*" element={<PlaceholderPage label="应用管理" />} />
        <Route path="nodes/*" element={<PlaceholderPage label="节点管理" />} />
        <Route path="settings" element={<PlaceholderPage label="系统设置" />} />
        <Route path="settings/users/*" element={<PlaceholderPage label="用户管理" />} />
        <Route path="settings/credentials/*" element={<PlaceholderPage label="SSH 凭证" />} />
        <Route path="settings/audit/*" element={<PlaceholderPage label="审计记录" />} />
      </Route>
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}
