import { http, HttpResponse } from "msw";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { AppProviders } from "../../app/AppProviders";
import type { AuthSnapshot } from "../auth/AuthContext";
import { server } from "../../test/server";
import { OverviewPage } from "./OverviewPage";

const administrator: AuthSnapshot = {
  status: "authenticated",
  csrfToken: "csrf-for-test",
  user: { id: "admin-1", username: "admin", displayName: "陈舟", identity: "administrator" },
};

const deployment = {
  id: "dep_1",
  target_id: "target_1",
  requested_by: "usr_1",
  status: "failed",
  phase: "failed",
  execution_mode: "script",
  stage_tasks: [],
  snapshot_hash: "sha256:abc",
  protocol_complete: true,
  queued_at: "2026-08-05T00:00:00Z",
  created_at: "2026-08-05T00:00:00Z",
  updated_at: "2026-08-05T00:00:00Z",
  version: 1,
  result_summary: "脚本缺少运行时依赖",
};
const target = {
  id: "target_1",
  application_id: "app_1",
  node_id: "node_1",
  environment: "生产",
  execution_mode: "script",
  script_path: "/srv/app/deploy.sh",
  parameter_schema: { type: "object", required: ["release-version"], properties: { "release-version": { type: "string" } } },
  secret_file_references: [],
  snapshot_hash: "sha256:target",
  verification_config: {},
  timeout_seconds: 1200,
  status: "active",
  created_at: "2026-08-05T00:00:00Z",
  updated_at: "2026-08-05T00:00:00Z",
  version: 1,
};
const node = {
  id: "node_1",
  name: "prod-01",
  status: "offline",
  created_at: "2026-08-05T00:00:00Z",
  updated_at: "2026-08-05T00:00:00Z",
  version: 1,
};
const app = {
  id: "app_1",
  name: "Billing",
  slug: "billing",
  description: "",
  status: "active",
  created_at: "2026-08-05T00:00:00Z",
  updated_at: "2026-08-05T00:00:00Z",
  version: 1,
};

function renderOverview() {
  return render(
    <MemoryRouter initialEntries={["/overview"]}>
      <AppProviders initialAuth={administrator}>
        <OverviewPage />
      </AppProviders>
    </MemoryRouter>,
  );
}

describe("概览页", () => {
  it("渲染运行摘要、最近活动和需要关注", async () => {
    server.use(
      http.get("/api/v1/deployments", () => HttpResponse.json({ items: [deployment], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [node], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [app], next_cursor: null })),
      http.get("/api/v1/deployment-targets/target_1", () => HttpResponse.json(target)),
    );
    renderOverview();

    expect(await screen.findByText("运行中的部署")).toBeInTheDocument();
    expect(screen.getByText("失败待处理")).toBeInTheDocument();
    expect(screen.getByText("异常节点")).toBeInTheDocument();
    expect(screen.getByText("已配置应用")).toBeInTheDocument();
    expect(screen.getAllByText("Billing").length).toBeGreaterThan(0);
    expect(screen.getByRole("link", { name: /节点 prod-01 离线/ })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Billing 部署失败/ })).toBeInTheDocument();
    expect(screen.getByText("脚本缺少运行时依赖")).toBeInTheDocument();
    expect(screen.getByText("查看全部").closest("a")).toHaveAttribute("href", "/deployments");
  });

  it("没有节点时显示空状态而不是持续加载", async () => {
    server.use(
      http.get("/api/v1/deployments", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [], next_cursor: null })),
    );
    renderOverview();

    expect(await screen.findByRole("heading", { name: "暂无数据" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "正在加载" })).not.toBeInTheDocument();
  });

  it("接口失败时显示错误并提供重试", async () => {
    server.use(
      http.get("/api/v1/deployments", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ code: "internal_error", message: "服务暂时不可用", request_id: "req-overview" }, { status: 500 })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [], next_cursor: null })),
    );
    renderOverview();

    expect(await screen.findByText("服务暂时不可用", undefined, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
  });
});
