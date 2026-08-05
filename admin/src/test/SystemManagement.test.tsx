import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { vi } from "vitest";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-system", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const ordinaryUser = { id: "user-1", username: "operator", display_name: "部署用户", email: "operator@example.invalid", identity: "user", status: "active", version: 1 };

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("用户管理", () => {
  it("管理员设置初始密码创建普通用户且没有邀请或角色入口", async () => {
    let body: unknown;
    server.use(
      http.get("/api/v1/users", () => HttpResponse.json({ items: [ordinaryUser], next_cursor: null })),
      http.post("/api/v1/users", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system"); body = await request.json(); return HttpResponse.json(ordinaryUser, { status: 201 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/users");
    await screen.findByText("部署用户");
    expect(screen.queryByText(/邀请用户|角色管理/)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "创建用户" }));
    await user.type(screen.getByLabelText("用户名"), "release");
    await user.type(screen.getByLabelText("显示名称"), "发布用户");
    await user.type(screen.getByLabelText("邮箱"), "release@example.invalid");
    await user.type(screen.getByLabelText("初始密码"), "initial-pass-123");
    await user.click(screen.getAllByRole("button", { name: "创建用户" }).at(-1)!);
    await waitFor(() => expect(body).toEqual({ username: "release", display_name: "发布用户", email: "release@example.invalid", password: "initial-pass-123" }));
    expect(screen.queryByDisplayValue("initial-pass-123")).not.toBeInTheDocument();
  });

  it("重置密码携带版本且完成后清除密码草稿", async () => {
    let resetBody: unknown;
    server.use(
      http.get("/api/v1/users/user-1", () => HttpResponse.json(ordinaryUser)),
      http.post("/api/v1/users/user-1/password", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system"); resetBody = await request.json(); return new HttpResponse(null, { status: 204 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/users/user-1");
    await user.click(await screen.findByRole("button", { name: "重置密码" }));
    await user.type(screen.getByLabelText(/^新密码/), "replacement-123");
    await user.click(screen.getByRole("button", { name: "确认重置" }));
    await waitFor(() => expect(resetBody).toEqual({ password: "replacement-123", version: 1 }));
    expect(screen.queryByLabelText(/^新密码/)).not.toBeInTheDocument();
  });
});

describe("系统设置与审计", () => {
  it("保存设置时携带 CSRF 和当前版本", async () => {
    const initial = { max_concurrent_deployments: 2, max_log_bytes: 52428800, log_retention_days: 30, version: 1 };
    let saved: unknown;
    let releaseUpdate!: () => void;
    const updateGate = new Promise<void>((resolve) => { releaseUpdate = resolve; });
    server.use(
      http.get("/api/v1/settings", () => HttpResponse.json(initial)),
      http.patch("/api/v1/settings", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system"); saved = await request.json(); await updateGate; return HttpResponse.json({ ...initial, max_concurrent_deployments: 4, version: 2 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/settings");
    const concurrency = await screen.findByLabelText(/^最大并发部署数/);
    fireEvent.change(concurrency, { target: { value: "4" } });
    await user.click(screen.getByRole("button", { name: "保存设置" }));
    expect(concurrency).toBeDisabled();
    expect(screen.getByRole("button", { name: "丢弃草稿" })).toBeDisabled();
    releaseUpdate();
    await waitFor(() => expect(saved).toEqual({ max_concurrent_deployments: 4, max_log_bytes: 52428800, log_retention_days: 30, version: 1 }));
    expect(concurrency).toHaveValue(4);
  });

  it("审计筛选变化从第一页重新请求", async () => {
    const requests: string[] = [];
    server.use(http.get("/api/v1/audit-logs", ({ request }) => { const url = new URL(request.url); requests.push(url.search); return HttpResponse.json({ items: [{ id: "aud-1", actor_id: "admin-1", action: "user.create", resource_type: "user", resource_id: "user-1", request_id: "req-audit", summary: {}, created_at: "2026-08-02T00:00:00Z" }], next_cursor: url.searchParams.has("action") ? null : "next-audit" }); }));
    const user = userEvent.setup();
    renderRoute("/settings/audit");
    await screen.findByText("user.create");
    await user.type(screen.getByLabelText("动作"), "user.create");
    await waitFor(() => expect(requests.at(-1)).toContain("action=user.create"));
    expect(requests.at(-1)).not.toContain("after=");
  });
});

describe("Agent 版本管理", () => {
  it("列出版本并只允许清理历史版本", async () => {
    let releases = {
      current_version: "0.1.0",
      items: [
        { version: "0.1.0", active: true, protocol_minimum: 1, protocol_maximum: 1 },
        { version: "0.2.0", active: false, protocol_minimum: 1, protocol_maximum: 1 },
      ],
    };
    let deleted: string | undefined;
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    server.use(
      http.get("/api/v1/agent/releases", () => HttpResponse.json(releases)),
      http.delete("/api/v1/agent/releases/:version", async ({ request, params }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system");
        deleted = String(params.version);
        releases = { ...releases, items: releases.items.filter((item) => item.version !== deleted) };
        return new HttpResponse(null, { status: 204 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/agent-releases");

    expect(await screen.findByRole("heading", { name: "Agent 版本" })).toBeInTheDocument();
    await screen.findByText("0.1.0");
    const cleanButtons = screen.getAllByRole("button", { name: "清理" });
    expect(cleanButtons[0]).toBeDisabled();
    expect(cleanButtons[1]).toBeEnabled();

    await user.click(cleanButtons[1]);

    await waitFor(() => expect(deleted).toBe("0.2.0"));
    expect(await screen.findByText("0.1.0")).toBeInTheDocument();
    expect(screen.queryByText("0.2.0")).not.toBeInTheDocument();
    confirmSpy.mockRestore();
  });
});

describe("我的", () => {
  it("偏好加载失败时显示错误而不是停留在加载状态", async () => {
    server.use(
      http.get("/api/v1/auth/profile", () => HttpResponse.json({ id: "user-1", username: "operator", display_name: "部署用户", email: null, identity: "user" })),
      http.get("/api/v1/auth/preferences", () => HttpResponse.json({ code: "preferences_unavailable", message: "偏好暂时不可用", request_id: "req-profile-error" }, { status: 503 })),
    );
    renderRoute("/profile", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByText("偏好暂时不可用", undefined, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByText("正在加载")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
  });

  it("资料与通知偏好保存后可从服务端重新恢复", async () => {
    let displayName = "部署用户";
    let preferences = { notify_deployment_failed: true, notify_deployment_completed: false, notify_node_unhealthy: true, time_format: "24h", follow_logs: true, version: 1 };
    server.use(
      http.get("/api/v1/auth/profile", () => HttpResponse.json({ id: "user-1", username: "operator", display_name: displayName, email: "operator@example.invalid", identity: "user" })),
      http.patch("/api/v1/auth/profile", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system"); const body = await request.json() as { display_name: string }; displayName = body.display_name; return HttpResponse.json({ id: "user-1", username: "operator", display_name: displayName, email: "operator@example.invalid", identity: "user" }); }),
      http.get("/api/v1/auth/preferences", () => HttpResponse.json(preferences)),
      http.put("/api/v1/auth/preferences", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-system"); const body = await request.json() as typeof preferences; preferences = { ...body, version: 2 }; return HttpResponse.json(preferences); }),
    );
    const user = userEvent.setup();
    const view = renderRoute("/profile", { ...administrator, user: { ...administrator.user!, id: "user-1", username: "operator", displayName: "部署用户", identity: "user" } });
    const name = await screen.findByLabelText("显示名称");
    await user.clear(name);
    await user.type(name, "值班用户");
    await user.click(screen.getByRole("button", { name: "保存资料" }));
    await user.click(screen.getByLabelText("部署完成"));
    await user.click(screen.getByLabelText("时间格式"));
    await user.click(await screen.findByRole("option", { name: "12 小时" }));
    await user.click(screen.getByRole("button", { name: "保存偏好" }));
    await waitFor(() => expect(preferences).toMatchObject({ notify_deployment_completed: true, time_format: "12h", version: 2 }));
    view.unmount();
    renderRoute("/profile", { ...administrator, user: { ...administrator.user!, id: "user-1", username: "operator", displayName: "值班用户", identity: "user" } });
    expect(await screen.findByLabelText("显示名称")).toHaveValue("值班用户");
    expect(screen.getByLabelText("部署完成")).toBeChecked();
    expect(screen.getByLabelText("时间格式")).toHaveTextContent("12 小时");
    expect(screen.queryByText(/权限说明/)).not.toBeInTheDocument();
  });
});
