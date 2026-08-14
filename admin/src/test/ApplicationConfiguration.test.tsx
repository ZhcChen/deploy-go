import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-apps", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const appOne = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", app_type: "binary", type_version: "1", environment: "prod", status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
const appTwo = { id: "app-2", name: "API Service", slug: "api-service", description: "API", environment: "test", status: "active", version: 1, created_at: "2026-08-01T00:00:01Z", updated_at: "2026-08-01T00:00:01Z" };
const archived = { ...appTwo, id: "app-archived", name: "Legacy", slug: "legacy", status: "archived" };

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("应用列表", () => {
  it("cursor 翻页去重且状态筛选从第一页重新请求", async () => {
    const requests: string[] = [];
    server.use(http.get("/api/v1/applications", ({ request }) => {
      const url = new URL(request.url);
      requests.push(url.search);
      if (url.searchParams.get("status") === "archived") return HttpResponse.json({ items: [archived], next_cursor: null });
      if (url.searchParams.get("after") === "cursor-1") return HttpResponse.json({ items: [appOne, appTwo], next_cursor: null });
      return HttpResponse.json({ items: [appOne], next_cursor: "cursor-1" });
    }));
    const user = userEvent.setup();
    renderRoute("/apps");
    expect(await screen.findByText("Voucher Hub")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "加载更多" }));
    expect(await screen.findByText("API Service")).toBeInTheDocument();
    expect(screen.getAllByText("Voucher Hub")).toHaveLength(1);
    await user.click(screen.getByLabelText("状态"));
    await user.click(await screen.findByRole("option", { name: "已归档" }));
    expect(await screen.findByText("Legacy")).toBeInTheDocument();
    expect(screen.queryByText("Voucher Hub")).not.toBeInTheDocument();
    expect(requests.at(-1)).toContain("status=archived");
    expect(requests.at(-1)).not.toContain("after=");
  });

  it("普通用户只显示服务端返回的授权应用且没有写入口", async () => {
    server.use(http.get("/api/v1/applications", () => HttpResponse.json({ items: [appOne], next_cursor: null })));
    renderRoute("/apps", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByText("Voucher Hub")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "创建应用" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "查看" })).toBeInTheDocument();
  });

  it("离开未保存的应用草稿前需要确认", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    const user = userEvent.setup();
    renderRoute("/apps");
    await user.click(screen.getByRole("button", { name: "创建应用" }));
    await user.type(screen.getByLabelText("应用名称"), "未保存应用");
    await user.click(screen.getByRole("link", { name: "节点" }));
    expect(confirm).toHaveBeenCalledWith("当前页面有未保存的修改，确定离开吗？");
    expect(screen.getByRole("heading", { level: 1, name: "应用" })).toBeInTheDocument();
  });
});

describe("部署目标", () => {
  it("应用详情目标列表展示节点名称、执行模式与特权 release 状态", async () => {
    server.use(
      http.get("/api/v1/applications/app-1", () => HttpResponse.json(appOne)),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [{ id: "target-1", application_id: "app-1", node_id: "node-1", target_code: "prod", environment: "production", execution_mode: "two_stage", script_path: "/srv/apps/voucher-hub/deploy.sh", parameter_schema: {}, secret_file_references: [], verification_config: {}, timeout_seconds: 900, status: "active", snapshot_hash: "snap-1", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }], next_cursor: null })),
      http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ code: "not_found", message: "应用来源不存在", request_id: "req-source-missing" }, { status: 404 })),
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [{ id: "node-1", name: "生产节点01", host: "node.fixture.invalid", status: "online", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }], next_cursor: null })),
    );
    renderRoute("/apps/app-1");
    expect(await screen.findByText("生产节点01")).toBeInTheDocument();
    expect(screen.getByText("普通二进制 v1")).toBeInTheDocument();
    expect(screen.getByText("两阶段")).toBeInTheDocument();
    expect(screen.getByText("原生特权 release")).toBeInTheDocument();
    expect(screen.getByText("prod")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "配置" })).toHaveAttribute("href", "/apps/app-1/targets/target-1");
  });

  it("目标详情展示节点摘要与特权 release 状态", async () => {
    server.use(
      http.get("/api/v1/applications/app-1", () => HttpResponse.json(appOne)),
      http.get("/api/v1/deployment-targets/target-1", () => HttpResponse.json({ id: "target-1", application_id: "app-1", node_id: "node-1", target_code: "prod", environment: "production", execution_mode: "two_stage", script_path: "/srv/apps/voucher-hub/deploy.sh", parameter_schema: {}, secret_file_references: [], verification_config: {}, timeout_seconds: 900, status: "active", snapshot_hash: "snap-1", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" })),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ application_id: "app-1", repository_url: "git@github.com:example/voucher-hub.git", ref_kind: "branch", deployment_branch: "production", git_credential_id: null, source_agent_id: "agent-1", version: 1, updated_at: "2026-08-02T00:00:00Z" })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [{ id: "node-1", name: "生产节点01", host: "node.fixture.invalid", status: "online", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }], next_cursor: null })),
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ id: "node-1", name: "生产节点01", host: "node.fixture.invalid", status: "online", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" })),
    );
    renderRoute("/apps/app-1/targets/target-1");
    expect(await screen.findByRole("heading", { name: "生产节点01" })).toBeInTheDocument();
    expect(screen.getByText("原生特权 release")).toBeInTheDocument();
    expect(screen.getByText("两阶段")).toBeInTheDocument();
  });

  it("应用契约无效 JSON 保留草稿并阻止提交", async () => {
    let updateCalls = 0;
    server.use(
      http.get("/api/v1/applications/app-1", () => HttpResponse.json(appOne)),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ code: "not_found", message: "应用来源不存在", request_id: "req-source-missing" }, { status: 404 })),
      http.put("/api/v1/applications/app-1", () => { updateCalls += 1; return HttpResponse.json(appOne); }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");
    await user.click(await screen.findByRole("button", { name: "编辑应用" }));
    fireEvent.change(await screen.findByLabelText(/参数 JSON Schema/), { target: { value: "{invalid" } });
    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("参数 JSON Schema 不是有效 JSON");
    expect(screen.getByLabelText(/参数 JSON Schema/)).toHaveValue("{invalid");
    expect(updateCalls).toBe(0);
    await user.click(screen.getByRole("button", { name: "丢弃草稿" }));
    expect(screen.queryByLabelText(/参数 JSON Schema/)).not.toBeInTheDocument();
  });

  it("两阶段目标固定特权 release 并提交目标配置", async () => {
    let requestBody: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/v1/applications/app-1", () => HttpResponse.json(appOne)),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({
        application_id: "app-1",
        repository_url: "git@github.com:example/voucher-hub.git",
        ref_kind: "branch",
        deployment_branch: "production",
        git_credential_id: null,
        source_agent_id: "agent-1",
        version: 1,
        updated_at: "2026-08-01T00:00:00Z",
      })),
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [{ id: "node-1", name: "Node", host: "node.fixture.invalid", port: 22, username: "deploy", ssh_credential_id: "cred-1", work_root: "/srv/apps", secrets_root: "/srv/secrets", status: "online", trusted_host_fingerprint: "SHA256:host", checked_at: "2026-08-01T00:00:00Z", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }], next_cursor: null })),
      http.post("/api/v1/applications/app-1/targets", async ({ request }) => {
        requestBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "target-1", application_id: "app-1", ...requestBody, status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");
    await user.click(await screen.findByRole("button", { name: "添加目标" }));

    await user.click(screen.getByLabelText("节点"));
    await user.click(await screen.findByRole("option", { name: "Node · node.fixture.invalid" }));
    await user.click(screen.getByLabelText("执行模式"));
    await user.click(await screen.findByRole("option", { name: "两阶段模式（prepare + release）" }));
    expect(screen.queryByLabelText(/敏感文件引用（旧版单脚本模式）/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "保存目标" }));
    expect(requestBody).toMatchObject({
      execution_mode: "two_stage",
      node_id: "node-1",
    });
    expect(requestBody).not.toHaveProperty("privileged_release");
    expect(requestBody).not.toHaveProperty("privileged_release_confirmed");
  });

  it("镜像直连目标选择模板、镜像、宿主端口与 Env 文件后提交配置", async () => {
    let requestBody: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/v1/applications/app-1", () => HttpResponse.json(appOne)),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [
        { id: "env-compose", file_name: "compose.env", module: "compose", format: "dotenv-v1", current_version: 1, current_digest: "a".repeat(64), declared_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z", target_count: 0, pending_count: 0, syncing_count: 0, succeeded_count: 0, failed_count: 0, syncs: [] },
        { id: "env-redis", file_name: "redis.env", module: "redis", format: "dotenv-v1", current_version: 1, current_digest: "a".repeat(64), declared_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z", target_count: 0, pending_count: 0, syncing_count: 0, succeeded_count: 0, failed_count: 0, syncs: [] },
      ], next_cursor: null })),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ code: "not_found", message: "应用来源不存在", request_id: "req-source-missing" }, { status: 404 })),
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [{ id: "node-1", name: "Node", host: "node.fixture.invalid", port: 22, username: "deploy", ssh_credential_id: "cred-1", work_root: "/srv/apps", secrets_root: "/srv/secrets", status: "online", trusted_host_fingerprint: "SHA256:host", checked_at: "2026-08-01T00:00:00Z", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }], next_cursor: null })),
      http.post("/api/v1/applications/app-1/targets", async ({ request }) => {
        requestBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "target-image", application_id: "app-1", ...requestBody, status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");
    await user.click(await screen.findByRole("button", { name: "添加目标" }));
    await user.click(screen.getByLabelText("节点"));
    await user.click(await screen.findByRole("option", { name: "Node · node.fixture.invalid" }));
    await user.click(screen.getByLabelText("执行模式"));
    await user.click(await screen.findByRole("option", { name: "镜像直连模式（模板 + 官方镜像）" }));
    expect(screen.queryByLabelText(/敏感文件引用（旧版单脚本模式）/)).not.toBeInTheDocument();
    expect(screen.getByLabelText("镜像引用")).toHaveValue("docker.io/library/redis:7-alpine");
    await user.click(await screen.findByRole("checkbox", { name: /compose\.env/ }));
    await user.click(await screen.findByRole("checkbox", { name: /redis\.env/ }));
    await user.click(screen.getByRole("button", { name: "保存目标" }));
    expect(requestBody).toMatchObject({
      execution_mode: "image",
      node_id: "node-1",
      image_spec: { template: "redis", image: "docker.io/library/redis:7-alpine", host_port: 6379, env_files: ["compose.env", "redis.env"] },
    });
    expect(requestBody).not.toHaveProperty("privileged_release");
    expect(requestBody).not.toHaveProperty("privileged_release_confirmed");
    expect(requestBody!.secret_file_references).toEqual([]);
  });
});

describe("应用授权", () => {
  it("分配和撤销后立即刷新显式授权集合", async () => {
    let granted = false;
    server.use(
      http.get("/api/v1/users", () => HttpResponse.json({ items: [{ id: "user-1", username: "operator", display_name: "部署用户", identity: "user", status: "active", version: 1 }], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [appOne], next_cursor: null })),
      http.get("/api/v1/users/user-1/applications", () => HttpResponse.json({ items: granted ? [{ application_id: "app-1", granted_at: "2026-08-01T00:00:00Z" }] : [], next_cursor: null })),
      http.put("/api/v1/users/user-1/applications/app-1", ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-apps"); granted = true; return new HttpResponse(null, { status: 204 }); }),
      http.delete("/api/v1/users/user-1/applications/app-1", () => { granted = false; return new HttpResponse(null, { status: 204 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/application-access");
    await user.click(await screen.findByRole("button", { name: /部署用户/ }));
    const item = await screen.findByRole("button", { name: /Voucher Hub/ });
    expect(item).toHaveAttribute("aria-pressed", "false");
    await user.click(item);
    expect(await screen.findByRole("button", { name: /Voucher Hub/ })).toHaveAttribute("aria-pressed", "true");
    expect(within(screen.getByRole("button", { name: /Voucher Hub/ })).getByText("已分配")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Voucher Hub/ }));
    expect(await screen.findByRole("button", { name: /Voucher Hub/ })).toHaveAttribute("aria-pressed", "false");
  });

  it("停用用户只能撤销既有授权", async () => {
    let grantCalls = 0;
    let revokeCalls = 0;
    server.use(
      http.get("/api/v1/users", () => HttpResponse.json({ items: [{ id: "user-disabled", username: "disabled", display_name: "停用用户", identity: "user", status: "disabled", version: 1 }], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [appOne, appTwo], next_cursor: null })),
      http.get("/api/v1/users/user-disabled/applications", () => HttpResponse.json({ items: revokeCalls === 0 ? [{ application_id: "app-1", granted_at: "2026-08-01T00:00:00Z" }] : [], next_cursor: null })),
      http.put("/api/v1/users/user-disabled/applications/:applicationId", () => { grantCalls += 1; return new HttpResponse(null, { status: 204 }); }),
      http.delete("/api/v1/users/user-disabled/applications/app-1", () => { revokeCalls += 1; return new HttpResponse(null, { status: 204 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/application-access");
    await user.click(await screen.findByRole("button", { name: /停用用户/ }));

    const granted = await screen.findByRole("button", { name: /Voucher Hub/ });
    const unassigned = screen.getByRole("button", { name: /API Service/ });
    expect(granted).toBeEnabled();
    expect(unassigned).toBeDisabled();
    await user.click(unassigned);
    expect(grantCalls).toBe(0);

    await user.click(granted);
    expect(revokeCalls).toBe(1);
    expect(await screen.findByRole("button", { name: /Voucher Hub/ })).toHaveAttribute("aria-pressed", "false");
  });

  it("自动读完授权游标后再开放操作", async () => {
    server.use(
      http.get("/api/v1/users", () => HttpResponse.json({ items: [{ id: "user-1", username: "operator", display_name: "部署用户", identity: "user", status: "active", version: 1 }], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [appTwo], next_cursor: null })),
      http.get("/api/v1/users/user-1/applications", ({ request }) => new URL(request.url).searchParams.get("after") === "grant-page-2"
        ? HttpResponse.json({ items: [{ application_id: "app-2", granted_at: "2026-08-01T00:00:01Z" }], next_cursor: null })
        : HttpResponse.json({ items: [], next_cursor: "grant-page-2" })),
    );
    const user = userEvent.setup();
    renderRoute("/settings/application-access");
    await user.click(await screen.findByRole("button", { name: /部署用户/ }));
    const item = await screen.findByRole("button", { name: /API Service/ });
    expect(item).toHaveAttribute("aria-pressed", "true");
    expect(item).toBeEnabled();
  });
});
