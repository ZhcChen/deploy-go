import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-workspace", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const application = { id: "app-1", name: "ClickHouse", slug: "clickhouse", description: "分析数据库", environment: "test", status: "active", version: 1, created_at: "2026-09-03T00:00:00Z", updated_at: "2026-09-03T00:00:00Z" };
const agent = { id: "agent-1", name: "Build Agent", node_id: "node-1", environment: "测试", status: "online", protocol_version: 14, agent_version: "0.2.0", created_at: "2026-09-03T00:00:00Z" };
const workspaceSourceMissing = { code: "not_found", message: "工作区来源不存在", request_id: "req-workspace-source-missing" };
const savedWorkspaceSource = {
  id: "workspace_source_1",
  application_id: "app-1",
  build_agent_id: "agent-1",
  build_agent_name: "Build Agent",
  workspace_path: "/srv/workspaces/clickhouse",
  workspace_version: 1,
  status: "verified",
  created_by: "admin-1",
  created_at: "2026-09-03T00:00:00Z",
  updated_at: "2026-09-03T00:00:00Z",
  version: 1,
};

function renderRoute(path: string) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={administrator}><RouterProvider router={router} /></AppProviders>);
}

function baseHandlers() {
  return [
    http.get("/api/v1/applications/app-1", () => HttpResponse.json(application)),
    http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ code: "not_found", message: "Git 来源不存在", request_id: "req-source-missing" }, { status: 404 })),
    http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
  ];
}

describe("本地工作区来源配置", () => {
  it("未配置时只显示开始按钮，管理员保存后进入已配置状态", async () => {
    let saveBody: unknown;
    server.use(
      ...baseHandlers(),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/applications/app-1/workspace-source", () => HttpResponse.json(workspaceSourceMissing, { status: 404 })),
      http.put("/api/v1/applications/app-1/workspace-source", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-workspace");
        saveBody = await request.json();
        return HttpResponse.json(savedWorkspaceSource);
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");

    expect(await screen.findByRole("button", { name: "开始配置工作区" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "配置工作区来源" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "开始配置工作区" }));

    await user.click(await screen.findByLabelText("构建节点"));
    await user.click(await screen.findByRole("option", { name: /Build Agent · v0\.2\.0/ }));
    await user.type(await screen.findByLabelText(/工作区路径/), "/srv/workspaces/clickhouse");
    const sourceForm = screen.getByLabelText(/工作区路径/).closest("form");
    if (!sourceForm) throw new Error("工作区来源表单未渲染");
    fireEvent.submit(sourceForm);

    expect(await screen.findByText("/srv/workspaces/clickhouse")).toBeInTheDocument();
    expect(screen.getByText("Build Agent")).toBeInTheDocument();
    expect(screen.getByText("v1")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "配置工作区来源" })).toBeInTheDocument();
    expect(saveBody).toEqual({
      build_agent_id: "agent-1",
      workspace_path: "/srv/workspaces/clickhouse",
    });
  });
});
