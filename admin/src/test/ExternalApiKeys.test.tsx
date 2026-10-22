import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { vi } from "vitest";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };

const applications = [
  { id: "app-1", name: "卡券系统", slug: "voucher-hub", description: "", environment: "prod", status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" },
  { id: "app-2", name: "PostgreSQL", slug: "postgres", description: "", environment: "test", status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" },
];

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("对外 API Key 管理", () => {
  it("列出现有 Key 并可更新绑定应用", async () => {
    const key = { id: "ekey-1", name: "CI Key", status: "active", application_ids: ["app-1"], expires_at: null, last_used_at: null, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z", version: 1 };
    let updateBody: unknown;
    server.use(
      http.get("/api/v1/external-api-keys", () => HttpResponse.json({ items: [key], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: applications, next_cursor: null })),
      http.put("/api/v1/external-api-keys/ekey-1/applications", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf");
        updateBody = await request.json();
        key.application_ids = (updateBody as { application_ids: string[] }).application_ids;
        return HttpResponse.json(key);
      }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/external-api-keys");

    expect(await screen.findByRole("heading", { level: 2, name: "对外 API Key" })).toBeInTheDocument();
    expect(await screen.findByText("CI Key")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "管理应用" }));
    await user.click(await screen.findByRole("button", { name: /PostgreSQL/ }));
    await user.click(screen.getByRole("button", { name: "保存应用" }));

    await waitFor(() => expect(updateBody).toEqual({ application_ids: ["app-1", "app-2"] }));
    expect(await screen.findByText("2 个应用")).toBeInTheDocument();
  });

  it("创建 Key 后明文只显示一次并携带 CSRF", async () => {
    let createBody: unknown;
    server.use(
      http.get("/api/v1/external-api-keys", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: applications, next_cursor: null })),
      http.post("/api/v1/external-api-keys", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf");
        createBody = await request.json();
        return HttpResponse.json({ id: "ekey-2", name: "外部 CI", token: "dgx_secret_only_once", status: "active", application_ids: ["app-1"], expires_at: null, created_at: "2026-08-11T00:00:00Z" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/external-api-keys");

    await user.click(await screen.findByRole("button", { name: "创建 API Key" }));
    await user.type(screen.getByLabelText("Key 名称"), "外部 CI");
    await user.click(screen.getByRole("button", { name: /卡券系统/ }));
    await user.click(screen.getByRole("button", { name: "创建 Key" }));

    await waitFor(() => expect(createBody).toEqual({ name: "外部 CI", application_ids: ["app-1"], expires_at: null }));
    expect(await screen.findByText("dgx_secret_only_once")).toBeInTheDocument();
    expect(screen.getByText("API Key 已创建，明文只显示这一次")).toBeInTheDocument();
  });

  it("吊销 Key 需确认并立即在列表中标记", async () => {
    const key = { id: "ekey-1", name: "CI Key", status: "active", application_ids: ["app-1"], expires_at: null, last_used_at: null, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z", version: 1 };
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    server.use(
      http.get("/api/v1/external-api-keys", () => HttpResponse.json({ items: [key], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: applications, next_cursor: null })),
      http.post("/api/v1/external-api-keys/ekey-1/revoke", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf");
        key.status = "disabled";
        return HttpResponse.json(key);
      }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/external-api-keys");

    await screen.findByText("CI Key");
    await user.click(screen.getByRole("button", { name: "吊销" }));

    expect(await screen.findByText("已吊销")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "吊销" })).toBeDisabled();
    confirmSpy.mockRestore();
  });

  it("为 Key 限定可用节点，未配置时显示不限制", async () => {
    const key = { id: "ekey-1", name: "CI Key", status: "active", application_ids: ["app-1"], node_ids: [] as string[], expires_at: null, last_used_at: null, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z", version: 1 };
    const nodes = [
      { id: "node-a", name: "构建节点 A", status: "online", archived_at: null },
      { id: "node-b", name: "构建节点 B", status: "online", archived_at: null },
    ];
    let updateBody: unknown;
    server.use(
      http.get("/api/v1/external-api-keys", () => HttpResponse.json({ items: [key], next_cursor: null })),
      http.get("/api/v1/applications", () => HttpResponse.json({ items: applications, next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: nodes, next_cursor: null })),
      http.put("/api/v1/external-api-keys/ekey-1/nodes", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf");
        updateBody = await request.json();
        key.node_ids = (updateBody as { node_ids: string[] }).node_ids;
        return HttpResponse.json(key);
      }),
    );
    const user = userEvent.setup();
    renderRoute("/settings/external-api-keys");

    await screen.findByText("CI Key");
    expect(screen.getByText("不限制")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "管理节点" }));
    expect(await screen.findByText(/不勾选任何节点表示不限制/)).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: /构建节点 B/ }));
    await user.click(screen.getByRole("button", { name: "保存节点" }));

    await waitFor(() => expect(updateBody).toEqual({ node_ids: ["node-b"] }));
    expect(await screen.findByText("1 个节点")).toBeInTheDocument();
  });
});
