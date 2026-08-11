import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-deploy", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const application = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const target = { id: "target-1", application_id: "app-1", node_id: "node-1", environment: "production", script_path: "scripts/deploy.sh", parameter_schema: { type: "object", required: ["release-version"], properties: { "release-version": { type: "string", title: "发布版本" }, "no-build": { type: "boolean", title: "跳过构建" } } }, secret_file_references: [], verification_config: {}, timeout_seconds: 600, status: "active", snapshot_hash: "target-snapshot", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const targetTwo = { ...target, id: "target-2", node_id: "node-2", script_path: "scripts/deploy-secondary.sh" };
const runOne = { id: "run-1", target_id: "target-1", node_id: "node-1", agent_id: "agent-1", status: "succeeded", phase: "release", env_gate_status: "ready", result_summary: "发布完成", error_code: null, source_run_id: null, started_at: "2026-08-02T00:00:01Z", finished_at: "2026-08-02T00:00:10Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:10Z" };
const runTwo = { id: "run-2", target_id: "target-2", node_id: "node-2", agent_id: "agent-2", status: "running", phase: "artifact_download", env_gate_status: "pending", result_summary: null, error_code: null, source_run_id: null, started_at: "2026-08-02T00:00:02Z", finished_at: null, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:02Z" };
const deployment = { id: "deployment-1", application_id: "app-1", target_id: "target-1", target_runs: [runOne, runTwo], requested_by: "admin-1", status: "running", phase: "targets_running", execution_mode: "script", stage_tasks: [], snapshot_hash: "preview-snapshot", protocol_complete: false, queued_at: "2026-08-02T00:00:00Z", started_at: "2026-08-02T00:00:01Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:01Z", version: 1 };
const twoStageTarget = { id: "target-2", application_id: "app-1", node_id: "node-1", environment: "production", execution_mode: "two_stage", script_path: "/srv/app/deploy.sh", parameter_schema: { type: "object", required: ["release-version", "modules"], properties: { "release-version": { type: "string", maxLength: 32 }, modules: { type: "string", maxLength: 512, "x-options": ["worker", "api"] } }, additionalProperties: false }, secret_file_references: [], verification_config: {}, timeout_seconds: 900, status: "active", snapshot_hash: "target-two-stage", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const twoStageCommit = "0123456789abcdef0123456789abcdef01234567";
const twoStagePreview = { application_id: "app-1", application_name: "Voucher Hub", execution_mode: "two_stage", deployment_branch: "main", resolved_commit_sha: twoStageCommit, release_version: "20260806120000", modules: ["api", "worker"], parameters: {}, snapshot_hash: "preview-two-stage", targets: [{ target_id: "target-2", node_id: "node-1", node_name: "prod-01", agent_id: "agent-1", agent_online: true, env_gate_status: "not_required", script_path: "/srv/app/deploy.sh" }] };
const twoStageDeployment = { id: "deployment-two-stage", application_id: "app-1", target_id: "target-2", target_runs: [{ ...runTwo, id: "run-two-stage", target_id: "target-2", node_id: "node-1", status: "running", phase: "release", env_gate_status: "ready" }], requested_by: "admin-1", status: "running", phase: "deploying", execution_mode: "two_stage", deployment_branch: "main", resolved_commit_sha: twoStageCommit, release_version: "20260806120000", modules: ["api", "worker"], stage_tasks: [
  { task_id: "task-prepare", stage: "prepare", status: "succeeded", exit_code: 0, error_code: null, started_at: "2026-08-02T00:00:01Z", finished_at: "2026-08-02T00:00:10Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:10Z" },
  { task_id: "task-release", stage: "release", status: "running", exit_code: null, error_code: null, started_at: "2026-08-02T00:00:11Z", finished_at: null, created_at: "2026-08-02T00:00:11Z", updated_at: "2026-08-02T00:00:12Z" },
], snapshot_hash: "snapshot-two-stage", protocol_complete: false, queued_at: "2026-08-02T00:00:00Z", started_at: "2026-08-02T00:00:01Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:12Z", version: 1 };
const imageSpec = { template: "redis", image: "docker.io/library/redis:7-alpine", host_port: 6379, env_files: ["compose.env", "redis.env"] };
const imageTarget = { ...twoStageTarget, id: "target-image", execution_mode: "image", script_path: "", parameter_schema: {}, privileged_release: true, image_spec: imageSpec, snapshot_hash: "target-image" };
const imagePreview = { application_id: "app-1", application_name: "Voucher Hub", execution_mode: "image", image_spec: imageSpec, release_version: "20260806130000", resolved_commit_sha: "a".repeat(40), modules: null, parameters: {}, snapshot_hash: "preview-image", release_strategy: "automatic", targets: [{ target_id: "target-image", node_id: "node-1", node_name: "prod-01", agent_id: "agent-1", agent_online: true, env_gate_status: "ready", script_path: "", image_spec: imageSpec }] };
const imageDeployment = { ...twoStageDeployment, id: "deployment-image", target_id: "target-image", execution_mode: "image", image_spec: imageSpec, deployment_branch: null, modules: null, resolved_commit_sha: "a".repeat(40), release_version: "20260806130000", target_runs: [{ ...runOne, id: "run-image", target_id: "target-image", status: "running", phase: "release" }], stage_tasks: [{ task_id: "task-image-release", stage: "release", status: "running", exit_code: null, error_code: null, started_at: "2026-08-02T00:00:01Z", finished_at: null, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:02Z" }], snapshot_hash: "snapshot-image" };

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  let queryClient: QueryClient | undefined;
  const view = render(<AppProviders initialAuth={snapshot}><QueryClientCapture onCapture={(client) => { queryClient = client; }} /><RouterProvider router={router} /></AppProviders>);
  if (!queryClient) throw new Error("QueryClient 未初始化");
  return { ...view, router, queryClient };
}

function QueryClientCapture({ onCapture }: { onCapture(client: QueryClient): void }) {
  onCapture(useQueryClient());
  return null;
}

describe("Web 部署主闭环", () => {
  beforeEach(() => {
    server.use(http.get("/api/v1/deployments/:id/events", () => HttpResponse.json({ items: [], next_cursor: null })));
  });
  it("按应用预览全部目标并使用稳定幂等键只创建一个部署", async () => {
    const user = userEvent.setup();
    let previewBody: unknown;
    let confirmBody: unknown;
    let idempotencyKey = "";
    let confirms = 0;
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [target, targetTwo], next_cursor: null })),
      http.post("/api/v1/applications/app-1/deployment-preview", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-deploy"); previewBody = await request.json(); return HttpResponse.json({ application_id: "app-1", application_name: "Voucher Hub", execution_mode: "script", parameters: { "release-version": "v1.2.3", "no-build": true }, snapshot_hash: "preview-snapshot", targets: [
        { target_id: "target-1", node_id: "node-1", node_name: "prod-01", agent_id: "agent-1", agent_online: true, env_gate_status: "ready", script_path: "scripts/deploy.sh" },
        { target_id: "target-2", node_id: "node-2", node_name: "prod-02", agent_id: "agent-2", agent_online: false, env_gate_status: "pending", script_path: "scripts/deploy-secondary.sh" },
      ] }); }),
      http.post("/api/v1/applications/app-1/deployments", async ({ request }) => { confirms += 1; idempotencyKey = request.headers.get("Idempotency-Key") ?? ""; confirmBody = await request.json(); await new Promise((resolve) => setTimeout(resolve, 20)); return HttpResponse.json(deployment, { status: 201 }); }),
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-1/logs", () => new HttpResponse("event: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":0}\n\n", { headers: { "Content-Type": "text/event-stream" } })),
    );
    renderRoute("/deployments/new?application=app-1");
    await user.type(await screen.findByLabelText("发布版本"), "v1.2.3");
    await user.click(screen.getByLabelText("跳过构建"));
    await user.click(screen.getByRole("button", { name: "生成部署预览" }));
    expect(await screen.findByText("preview-snapshot")).toBeInTheDocument();
    expect(screen.getByText("prod-01")).toBeInTheDocument();
    expect(screen.getByText("prod-02")).toBeInTheDocument();
    expect(screen.getByText("Env 已就绪")).toBeInTheDocument();
    expect(screen.getByText("Env 等待同步")).toBeInTheDocument();
    expect(screen.getByText("离线，部署将等待节点恢复")).toBeInTheDocument();
    const confirm = screen.getByRole("button", { name: /确认并发起部署/ });
    await Promise.all([user.click(confirm), user.click(confirm)]);
    await user.click(await screen.findByRole("tab", { name: "日志" }));
    await screen.findByText("执行日志");
    expect(previewBody).toEqual({ parameters: { "release-version": "v1.2.3", "no-build": true }, release_strategy: "automatic" });
    expect(confirmBody).toEqual({ parameters: { "release-version": "v1.2.3", "no-build": true }, snapshot_hash: "preview-snapshot", release_strategy: "automatic" });
    expect(idempotencyKey).toMatch(/^deploy-[0-9a-f-]{36}$/);
    expect(confirms).toBe(1);
  });

  it("应用目标覆盖加载、零目标和 API 失败状态", async () => {
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", async () => { await new Promise((resolve) => setTimeout(resolve, 500)); return HttpResponse.json({ items: [], next_cursor: null }); }),
    );
    const emptyView = renderRoute("/deployments/new?application=app-1");
    await screen.findByLabelText("应用");
    expect(screen.getByText("正在加载")).toBeInTheDocument();
    expect(await screen.findByText("该应用没有可部署目标")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "生成部署预览" })).toBeDisabled();
    emptyView.unmount();

    server.use(http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ code: "targets_unavailable", message: "目标列表暂时不可用", request_id: "req-targets" }, { status: 503 })));
    renderRoute("/deployments/new?application=app-1");
    expect(await screen.findByText("目标列表暂时不可用", {}, { timeout: 2500 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "生成部署预览" })).toBeDisabled();
  });

  it("普通用户仍可发起已授权应用部署但不获得管理入口", async () => {
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [target], next_cursor: null })),
    );
    renderRoute("/deployments/new?application=app-1", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByRole("button", { name: "生成部署预览" })).toBeEnabled();
  });

  it("日志作为纯文本渲染、按游标续传并可取消", async () => {
    let cancelCalls = 0;
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-1/logs", ({ request }) => { const after = request.headers.get("Last-Event-ID"); return new HttpResponse(after ? "event: terminal\ndata: {\"status\":\"canceled\",\"last_event_id\":1}\n\n" : "id: 1\nevent: log\ndata: {\"sequence\":1,\"stream\":\"stdout\",\"content\":\"<img src=x onerror=alert(1)> javascript:evil()\\u0000\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:02Z\"}\n\nevent: future-event\ndata: <script>alert(1)</script>\n\n", { headers: { "Content-Type": "text/event-stream" } }); }),
      http.post("/api/v1/deployments/deployment-1/cancel", ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-deploy"); cancelCalls += 1; return HttpResponse.json({ ...deployment, status: "canceling", phase: "canceling", version: 2 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/deployments/deployment-1?view=logs");
    expect(await screen.findByText(/<img src=x onerror=alert\(1\)>/)).toBeInTheDocument();
    expect(await screen.findByText(/收到未知日志事件 future-event：<script>alert\(1\)<\/script>/)).toBeInTheDocument();
    expect(document.querySelector("main img")).toBeNull();
    expect(document.querySelector("main script")).toBeNull();
    await user.click(screen.getByRole("button", { name: "取消部署" }));
    await user.click(screen.getByRole("button", { name: "确认取消" }));
    await waitFor(() => expect(cancelCalls).toBe(1));
  });

  it("切换 deployment ID 时不复用上一部署的日志和游标", async () => {
    let deploymentTwoCursor: string | null | undefined;
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-2", () => HttpResponse.json({ ...deployment, id: "deployment-2" })),
      http.get("/api/v1/deployments/deployment-1/logs", () => new HttpResponse(
        "id: 120\nevent: log\ndata: {\"sequence\":120,\"stream\":\"stdout\",\"content\":\"deployment-one-output\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:02Z\"}\n\nevent: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":120}\n\n",
        { headers: { "Content-Type": "text/event-stream" } },
      )),
      http.get("/api/v1/deployments/deployment-2/logs", ({ request }) => {
        deploymentTwoCursor = request.headers.get("Last-Event-ID");
        return new HttpResponse(
          "id: 1\nevent: log\ndata: {\"sequence\":1,\"stream\":\"stdout\",\"content\":\"deployment-two-output\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:03Z\"}\n\nevent: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":1}\n\n",
          { headers: { "Content-Type": "text/event-stream" } },
        );
      }),
    );
    const view = renderRoute("/deployments/deployment-1?view=logs");
    expect(await screen.findByText("deployment-one-output")).toBeInTheDocument();

    await act(() => view.router.navigate("/deployments/deployment-2?view=logs"));

    expect(await screen.findByText("deployment-two-output")).toBeInTheDocument();
    expect(deploymentTwoCursor).toBeNull();
    expect(screen.queryByText("deployment-one-output")).not.toBeInTheDocument();
  });

  it("普通用户无权访问 deployment 时不请求 SSE 或泄露元数据", async () => {
    let logCalls = 0;
    server.use(
      http.get("/api/v1/deployments/secret-deployment", () => HttpResponse.json({ code: "forbidden", message: "没有部署访问权限", request_id: "req-forbidden" }, { status: 403 })),
      http.get("/api/v1/deployments/secret-deployment/logs", () => { logCalls += 1; return new HttpResponse(null, { status: 403 }); }),
    );
    renderRoute("/deployments/secret-deployment", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByText("没有部署访问权限", {}, { timeout: 2500 })).toBeInTheDocument();
    expect(screen.queryByText("secret-target")).not.toBeInTheDocument();
    expect(logCalls).toBe(0);
  });

  it("SSE 授权撤销后立即清除部署和已加载日志", async () => {
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-1/logs", () => HttpResponse.json(
        { code: "forbidden", message: "日志访问授权已失效", request_id: "req-sse-403" },
        { status: 403 },
      )),
    );
    renderRoute("/deployments/deployment-1?view=logs");

    expect(await screen.findByText("日志访问授权已失效")).toBeInTheDocument();
    expect(screen.queryByText("deployment-1")).not.toBeInTheDocument();
    expect(screen.queryByTestId("deployment-log")).not.toBeInTheDocument();
    expect(screen.getByText(/req-sse-403/)).toBeInTheDocument();
  });

  it("detail 返回 403 后删除详情和部署列表缓存", async () => {
    server.use(
      http.get("/api/v1/deployments/secret-deployment", () => HttpResponse.json(
        { code: "forbidden", message: "部署访问授权已失效", request_id: "req-detail-403" },
        { status: 403 },
      )),
    );
    const { queryClient } = renderRoute("/deployments/secret-deployment");
    queryClient.setQueryData(["deployments"], { items: [{ id: "cached-secret" }] });

    expect(await screen.findByText("部署访问授权已失效")).toBeInTheDocument();
    expect(queryClient.getQueryData(["deployment", "secret-deployment"])).toBeUndefined();
    expect(queryClient.getQueryData(["deployments"])).toBeUndefined();
  });

  it("cancel 或 retry 返回 403 后不保留部署内容", async () => {
    const user = userEvent.setup();
    let current = deployment;
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(current)),
      http.get("/api/v1/deployments/deployment-1/logs", () => new HttpResponse("", { headers: { "Content-Type": "text/event-stream" } })),
      http.post("/api/v1/deployments/deployment-1/cancel", () => HttpResponse.json(
        { code: "forbidden", message: "取消权限已撤销", request_id: "req-cancel-403" },
        { status: 403 },
      )),
      http.post("/api/v1/deployments/deployment-1/retry", () => HttpResponse.json(
        { code: "forbidden", message: "重试权限已撤销", request_id: "req-retry-403" },
        { status: 403 },
      )),
    );
    const view = renderRoute("/deployments/deployment-1");
    await user.click(await screen.findByRole("button", { name: "取消部署" }));
    await user.click(screen.getByRole("button", { name: "确认取消" }));
    expect(await screen.findByText("取消权限已撤销")).toBeInTheDocument();
    expect(screen.queryByText("deployment-1")).not.toBeInTheDocument();

    view.unmount();
    current = { ...deployment, status: "failed", phase: "failed", target_runs: [runOne, { ...runTwo, status: "failed", phase: "release" }] };
    renderRoute("/deployments/deployment-1?view=details");
    await user.click(await screen.findByRole("button", { name: "重试失败目标" }));
    await user.click(screen.getByRole("button", { name: /确认重试/ }));
    expect(await screen.findByText("重试权限已撤销")).toBeInTheDocument();
    expect(screen.queryByText("deployment-1")).not.toBeInTheDocument();
  });

  it("详情保留部分成功事实并在确认后只重试失败或未执行目标", async () => {
    const partialFailure = {
      ...deployment,
      status: "failed",
      phase: "targets_failed",
      finished_at: "2026-08-02T00:00:20Z",
      target_runs: [
        runOne,
        { ...runTwo, status: "failed", phase: "release", env_gate_status: "ready", error_code: "release_failed", result_summary: "发布脚本失败", finished_at: "2026-08-02T00:00:20Z" },
        { ...runTwo, id: "run-3", target_id: "target-3", node_id: "node-3", status: "expired", phase: "pending", env_gate_status: "pending", error_code: "target_offline", result_summary: "节点等待超时", finished_at: "2026-08-02T00:00:20Z" },
      ],
    };
    let retries = 0;
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(partialFailure)),
      http.get("/api/v1/deployments/deployment-1/logs", () => new HttpResponse("event: terminal\ndata: {\"status\":\"failed\",\"last_event_id\":0}\n\n", { headers: { "Content-Type": "text/event-stream" } })),
      http.post("/api/v1/deployments/deployment-1/retry", () => { retries += 1; return HttpResponse.json({ ...partialFailure, id: "deployment-retry", retry_of_id: "deployment-1", status: "queued", phase: "targets_pending" }, { status: 201 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/deployments/deployment-1?view=details");

    expect(await screen.findByRole("heading", { name: "逐节点状态" })).toBeInTheDocument();
    expect(screen.getByText("发布完成")).toBeInTheDocument();
    expect(screen.getByText("发布脚本失败")).toBeInTheDocument();
    expect(screen.getByText("节点等待超时")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重试失败目标" }));
    const dialog = screen.getByRole("dialog", { name: "重试失败目标" });
    expect(within(dialog).getByText("node-2")).toBeInTheDocument();
    expect(within(dialog).getByText("node-3")).toBeInTheDocument();
    expect(within(dialog).queryByText("node-1")).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "确认重试 2 个目标" }));
    await waitFor(() => expect(retries).toBe(1));
  });

  it("two_stage 预览展示固定分支、Commit、发布版本和模块", async () => {
    let previewBody: unknown;
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [twoStageTarget], next_cursor: null })),
      http.post("/api/v1/applications/app-1/deployment-preview", async ({ request }) => { previewBody = await request.json(); return HttpResponse.json({ ...twoStagePreview, release_strategy: "manual" }); }),
    );
    const user = userEvent.setup();
    renderRoute("/deployments/new?application=app-1");

    expect(await screen.findByText("已选择 2 / 2")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "worker" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "api" })).toBeChecked();
    expect(screen.queryByLabelText("发布版本")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "取消全选" }));
    expect(screen.getByRole("button", { name: "生成部署预览" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "全选" }));
    await user.click(await screen.findByRole("button", { name: "构建后手动发布" }));
    await user.click(screen.getByRole("button", { name: "生成部署预览" }));

    expect(await screen.findByText("两阶段（prepare + release）")).toBeInTheDocument();
    expect(screen.getByText("main")).toBeInTheDocument();
    expect(screen.getByText(twoStageCommit)).toBeInTheDocument();
    expect(screen.getByText("20260806120000")).toBeInTheDocument();
    expect(screen.getByText("api, worker")).toBeInTheDocument();
    expect(screen.getByText("preview-two-stage")).toBeInTheDocument();
    expect(previewBody).toEqual({ parameters: { modules: "worker,api" }, release_strategy: "manual" });
  });

  it("等待发布时可配置 Env 并确认开始 release", async () => {
    const user = userEvent.setup();
    let releases = 0;
    const waiting = { ...twoStageDeployment, phase: "awaiting_release", release_strategy: "manual", stage_tasks: [twoStageDeployment.stage_tasks[0]] };
    server.use(
      http.get("/api/v1/deployments/deployment-two-stage", () => HttpResponse.json(waiting)),
      http.post("/api/v1/deployments/deployment-two-stage/release", ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-deploy"); releases += 1; return HttpResponse.json({ ...waiting, phase: "deploying" }); }),
      http.get("/api/v1/deployments/deployment-two-stage/logs", () => new HttpResponse("", { headers: { "Content-Type": "text/event-stream" } })),
    );
    renderRoute("/deployments/deployment-two-stage?view=details");
    expect(await screen.findByText(/prepare 已完成/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "配置 Env" })).toHaveAttribute("href", "/applications/app-1");
    await user.click(screen.getByRole("button", { name: "开始发布" }));
    await user.click(within(screen.getByRole("dialog", { name: "开始发布" })).getByRole("button", { name: "确认开始发布" }));
    await waitFor(() => expect(releases).toBe(1));
  });

  it("two_stage 详情展示 prepare/release 阶段任务", async () => {
    server.use(
      http.get("/api/v1/deployments/deployment-two-stage", () => HttpResponse.json(twoStageDeployment)),
      http.get("/api/v1/deployments/deployment-two-stage/logs", () => new HttpResponse("event: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":0}\n\n", { headers: { "Content-Type": "text/event-stream" } })),
    );
    renderRoute("/deployments/deployment-two-stage?view=details");

    expect(await screen.findByText("阶段任务")).toBeInTheDocument();
    expect(screen.getByText("准备 prepare")).toBeInTheDocument();
    expect(screen.getByText("发布 release")).toBeInTheDocument();
    expect(screen.getByText("task-prepare")).toBeInTheDocument();
    expect(screen.getByText("task-release")).toBeInTheDocument();
    expect(screen.getAllByText("0")).toHaveLength(1);
    expect(screen.getByText("succeeded")).toBeInTheDocument();
    expect(screen.getByText("running")).toBeInTheDocument();
    expect(screen.getByText("main")).toBeInTheDocument();
    expect(screen.getByText(twoStageCommit)).toBeInTheDocument();
  });

  it("image 预览展示模板、镜像、宿主端口与 Env 文件且不展示 Git 信息", async () => {
    let previewBody: unknown;
    const user = userEvent.setup();
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [imageTarget], next_cursor: null })),
      http.post("/api/v1/applications/app-1/deployment-preview", async ({ request }) => { previewBody = await request.json(); return HttpResponse.json(imagePreview); }),
    );
    renderRoute("/deployments/new?application=app-1");

    expect(await screen.findByText(/镜像直连部署的镜像、模板、宿主端口与 Env 文件已由目标配置固定/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "生成部署预览" }));
    expect(await screen.findByText("镜像直连（固定 Make target）")).toBeInTheDocument();
    expect(screen.getByText("redis")).toBeInTheDocument();
    expect(screen.getAllByText("docker.io/library/redis:7-alpine").length).toBeGreaterThan(0);
    expect(screen.getByText("6379")).toBeInTheDocument();
    expect(screen.getByText("compose.env, redis.env")).toBeInTheDocument();
    expect(screen.getByText("preview-image")).toBeInTheDocument();
    expect(screen.queryByText("main")).not.toBeInTheDocument();
    expect(screen.queryByText(twoStageCommit)).not.toBeInTheDocument();
    expect(previewBody).toEqual({ parameters: {}, release_strategy: "automatic" });
  });

  it("image 详情展示镜像信息与 release 阶段任务且不展示 Git 分支", async () => {
    server.use(
      http.get("/api/v1/deployments/deployment-image", () => HttpResponse.json(imageDeployment)),
      http.get("/api/v1/deployments/deployment-image/logs", () => new HttpResponse("event: terminal\ndata: {\"status\":\"running\",\"last_event_id\":0}\n\n", { headers: { "Content-Type": "text/event-stream" } })),
    );
    renderRoute("/deployments/deployment-image?view=details");

    expect(await screen.findByText("镜像直连（固定 Make target）")).toBeInTheDocument();
    expect(screen.getByText("docker.io/library/redis:7-alpine")).toBeInTheDocument();
    expect(screen.getByText("6379")).toBeInTheDocument();
    expect(screen.getByText("compose.env, redis.env")).toBeInTheDocument();
    expect(screen.getByText("阶段任务")).toBeInTheDocument();
    expect(screen.getByText("发布 release")).toBeInTheDocument();
    expect(screen.getByText("task-image-release")).toBeInTheDocument();
    expect(screen.queryByText("main")).not.toBeInTheDocument();
  });
});
