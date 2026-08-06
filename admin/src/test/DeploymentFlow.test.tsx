import { act, render, screen, waitFor } from "@testing-library/react";
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
const deployment = { id: "deployment-1", target_id: "target-1", requested_by: "admin-1", status: "running", phase: "execute", snapshot_hash: "preview-snapshot", protocol_complete: false, queued_at: "2026-08-02T00:00:00Z", started_at: "2026-08-02T00:00:01Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:01Z", version: 1 };

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
  it("preview 后使用稳定幂等键确认且双击只创建一次", async () => {
    let previewBody: unknown;
    let confirmBody: unknown;
    let idempotencyKey = "";
    let confirms = 0;
    server.use(
      http.get("/api/v1/applications", () => HttpResponse.json({ items: [application], next_cursor: null })),
      http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [target], next_cursor: null })),
      http.post("/api/v1/deployment-targets/target-1/deployment-preview", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-deploy"); previewBody = await request.json(); return HttpResponse.json({ target_id: "target-1", application_id: "app-1", application_name: "Voucher Hub", node_id: "node-1", node_name: "prod-01", environment: "production", script_path: "scripts/deploy.sh", parameters: { "release-version": "v1.2.3", "no-build": true }, snapshot_hash: "preview-snapshot" }); }),
      http.post("/api/v1/deployment-targets/target-1/deployments", async ({ request }) => { confirms += 1; idempotencyKey = request.headers.get("Idempotency-Key") ?? ""; confirmBody = await request.json(); await new Promise((resolve) => setTimeout(resolve, 20)); return HttpResponse.json(deployment, { status: 201 }); }),
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-1/logs", () => new HttpResponse("event: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":0}\n\n", { headers: { "Content-Type": "text/event-stream" } })),
    );
    const user = userEvent.setup();
    renderRoute("/deployments/new?application=app-1&target=target-1");
    await user.type(await screen.findByLabelText("发布版本"), "v1.2.3");
    await user.click(screen.getByLabelText("跳过构建"));
    await user.click(screen.getByRole("button", { name: "生成部署预览" }));
    expect(await screen.findByText("preview-snapshot")).toBeInTheDocument();
    const confirm = screen.getByRole("button", { name: "确认并发起部署" });
    await Promise.all([user.click(confirm), user.click(confirm)]);
    await screen.findByText("执行日志");
    expect(previewBody).toEqual({ parameters: { "release-version": "v1.2.3", "no-build": true } });
    expect(confirmBody).toEqual({ parameters: { "release-version": "v1.2.3", "no-build": true }, snapshot_hash: "preview-snapshot" });
    expect(idempotencyKey).toMatch(/^deploy-[0-9a-f-]{36}$/);
    expect(confirms).toBe(1);
  });

  it("日志作为纯文本渲染、按游标续传并可取消", async () => {
    let cancelCalls = 0;
    server.use(
      http.get("/api/v1/deployments/deployment-1", () => HttpResponse.json(deployment)),
      http.get("/api/v1/deployments/deployment-1/logs", ({ request }) => { const after = request.headers.get("Last-Event-ID"); return new HttpResponse(after ? "event: terminal\ndata: {\"status\":\"canceled\",\"last_event_id\":1}\n\n" : "id: 1\nevent: log\ndata: {\"sequence\":1,\"stream\":\"stdout\",\"content\":\"<img src=x onerror=alert(1)> javascript:evil()\\u0000\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:02Z\"}\n\nevent: future-event\ndata: <script>alert(1)</script>\n\n", { headers: { "Content-Type": "text/event-stream" } }); }),
      http.post("/api/v1/deployments/deployment-1/cancel", ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-deploy"); cancelCalls += 1; return HttpResponse.json({ ...deployment, status: "canceling", phase: "canceling", version: 2 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/deployments/deployment-1");
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
    const view = renderRoute("/deployments/deployment-1");
    expect(await screen.findByText("deployment-one-output")).toBeInTheDocument();

    await act(() => view.router.navigate("/deployments/deployment-2"));

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
    renderRoute("/deployments/deployment-1");

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
    current = { ...deployment, status: "failed", phase: "failed" };
    renderRoute("/deployments/deployment-1");
    await user.click(await screen.findByRole("button", { name: "重试部署" }));
    expect(await screen.findByText("重试权限已撤销")).toBeInTheDocument();
    expect(screen.queryByText("deployment-1")).not.toBeInTheDocument();
  });
});
