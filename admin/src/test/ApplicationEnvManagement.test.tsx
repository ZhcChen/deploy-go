import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-env", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const application = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", environment: "prod", status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
const envSyncs = [
  { target_id: "target-pending", node_id: "node-pending", node_name: "Node Pending", status: "pending", actual_version: null, last_attempt_at: null, synced_at: null, error_code: null, error_message: null },
  { target_id: "target-success", node_id: "node-success", node_name: "Node Success", status: "succeeded", actual_version: 3, last_attempt_at: "2026-08-06T03:00:00Z", synced_at: "2026-08-06T03:00:01Z", error_code: null, error_message: null },
  { target_id: "target-failed", node_id: "node-failed", node_name: "Node Failed", status: "failed", actual_version: null, last_attempt_at: "2026-08-06T03:00:00Z", synced_at: null, error_code: "env_sync_digest_mismatch", error_message: "Env 同步失败" },
];
const envFile = { id: "env-1", application_id: "app-1", file_name: "api.env", module: "api", format: "dotenv-v1", current_version: 3, current_digest: "a".repeat(64), declared_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-06T03:00:00Z", version: 4, target_count: 3, pending_count: 1, syncing_count: 0, succeeded_count: 1, failed_count: 1, syncs: envSyncs };
const plaintext = { id: "env-1", application_id: "app-1", file_name: "api.env", module: "api", format: "dotenv-v1", content: "# API\nPORT=8080\nTOKEN=top-secret\n", digest: "a".repeat(64), env_version: 3, version: 4, updated_at: "2026-08-06T03:00:00Z" };

function mockApplicationShell() {
  server.use(
    http.get("/api/v1/applications/app-1", () => HttpResponse.json(application)),
    http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications/app-1/source", () => HttpResponse.json({ code: "not_found", message: "应用来源不存在", request_id: "req-source" }, { status: 404 })),
    http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [envFile] })),
  );
}

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("应用配置管理", () => {
  it("普通用户只能查看已有 Env 元数据且没有新建和明文入口", async () => {
    mockApplicationShell();
    renderRoute("/apps/app-1", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByRole("heading", { name: "应用配置" })).toBeInTheDocument();
    expect(await screen.findByText("api.env")).toBeInTheDocument();
    expect(screen.getByText("v3")).toBeInTheDocument();
    expect(screen.getByText("待同步 1")).toBeInTheDocument();
    expect(screen.getByText("Node Failed")).toBeInTheDocument();
    expect(screen.getByText("Env 同步失败")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "编辑 api.env" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "登记 Env" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "重试 Node Failed 的 Env 同步" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /新建 Env/ })).not.toBeInTheDocument();
    expect(screen.queryByText("top-secret")).not.toBeInTheDocument();
  });

  it("普通用户直接访问 Env 编辑地址返回 403", () => {
    renderRoute("/apps/app-1/config/env-1", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(screen.getByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
    expect(screen.queryByLabelText("管理员密码")).not.toBeInTheDocument();
  });

  it("普通用户直接访问 Env 登记地址返回 403", () => {
    renderRoute("/apps/app-1/config/new", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(screen.getByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
    expect(screen.queryByLabelText("管理员密码")).not.toBeInTheDocument();
  });

  it("空状态管理员可进入 Env 登记页", async () => {
    mockApplicationShell();
    server.use(http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [] })));
    renderRoute("/apps/app-1");
    expect(await screen.findByRole("heading", { name: "应用配置" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "登记 Env" })).toHaveAttribute("href", "/apps/app-1/config/new");
  });

  it("管理员重新验证后登记首个 Env 并同步到目标节点", async () => {
    mockApplicationShell();
    server.use(http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [] })));
    let registerBody: unknown;
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-env");
        expect(await request.json()).toEqual({ action: "read_write", password: "correct-password" });
        return HttpResponse.json({ action: "read_write", grant_token: "grant-register", expires_at: "2099-08-06T03:05:00Z" });
      }),
      http.post("/api/v1/applications/app-1/env-files/register", async ({ request }) => {
        expect(request.headers.get("X-Env-Reveal-Grant")).toBe("grant-register");
        registerBody = await request.json();
        return HttpResponse.json({ created: ["api.env"] }, { headers: { "Cache-Control": "no-store" } });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/new");
    expect(await screen.findByRole("heading", { name: "登记运行配置" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("文件名"), "api.env");
    await user.type(screen.getByLabelText("模块"), "api");
    await user.click(screen.getByRole("button", { name: "原文模式" }));
    await user.type(screen.getByLabelText("api.env 原文"), "# 首次登记\nSECRET=initial\nPORT=8080\n");
    await user.type(screen.getByLabelText("管理员密码"), "correct-password");
    await user.click(screen.getByRole("button", { name: "验证并继续登记" }));
    await user.click(await screen.findByRole("button", { name: "提交登记" }));
    const dialog = await screen.findByRole("dialog", { name: "登记 api.env？" });
    expect(dialog).toHaveTextContent("自动同步到全部启用目标节点");
    expect(dialog).not.toHaveTextContent("initial");
    expect(dialog).not.toHaveTextContent("8080");
    await user.click(within(dialog).getByRole("button", { name: "确认登记" }));
    expect(await screen.findByRole("heading", { name: "应用配置" })).toBeInTheDocument();
    expect(registerBody).toEqual({
      files: [{ file_name: "api.env", module: "api", format: "dotenv-v1", content: "# 首次登记\nSECRET=initial\nPORT=8080\n" }],
    });
  });

  it("重复文件名与非法 dotenv 阻止 Env 登记提交", async () => {
    server.use(http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [envFile] })));
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/new");
    await user.type(await screen.findByLabelText("文件名"), "api.env");
    expect(await screen.findByRole("alert")).toHaveTextContent("已存在同名配置");
    await user.clear(screen.getByLabelText("文件名"));
    await user.type(screen.getByLabelText("文件名"), "worker.env");
    await user.click(screen.getByRole("button", { name: "原文模式" }));
    await user.type(screen.getByLabelText("worker.env 原文"), "SECRET=duplicate\nSECRET=again\n");
    expect(await screen.findByRole("alert")).toHaveTextContent("原文校验未通过");
    expect(screen.queryByRole("button", { name: "提交登记" })).not.toBeInTheDocument();
  });

  it("管理员重新验证后读取明文并可切换结构化和原文模式", async () => {
    mockApplicationShell();
    let revealCalls = 0;
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-env");
        expect(await request.json()).toEqual({ action: "read_write", password: "correct-password" });
        return HttpResponse.json({ action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" });
      }),
      http.get("/api/v1/application-env-files/env-1", ({ request }) => {
        revealCalls += 1;
        expect(request.headers.get("X-Env-Reveal-Grant")).toBe("grant-read");
        return HttpResponse.json(plaintext, { headers: { "Cache-Control": "no-store" } });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/env-1");
    expect(await screen.findByRole("heading", { name: "重新验证管理员密码" })).toBeInTheDocument();
    expect(screen.queryByText("top-secret")).not.toBeInTheDocument();
    await user.type(screen.getByLabelText("管理员密码"), "correct-password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    expect(await screen.findByDisplayValue("8080")).toBeInTheDocument();
    expect(revealCalls).toBe(1);
    await user.click(screen.getByRole("button", { name: "原文模式" }));
    expect(screen.getByLabelText("api.env 原文")).toHaveValue(plaintext.content);
    await user.click(screen.getByRole("button", { name: "结构化模式" }));
    expect(screen.getByDisplayValue("8080")).toBeInTheDocument();
  });

  it("重复键关联具体行并阻止保存，确认 Diff 不泄漏值", async () => {
    mockApplicationShell();
    let updateCalls = 0;
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", () => HttpResponse.json({ action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" })),
      http.get("/api/v1/application-env-files/env-1", () => HttpResponse.json(plaintext)),
      http.put("/api/v1/application-env-files/env-1", () => { updateCalls += 1; return HttpResponse.json(plaintext); }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/env-1");
    await user.type(await screen.findByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    await user.click(await screen.findByRole("button", { name: "原文模式" }));
    fireEvent.change(screen.getByLabelText("api.env 原文"), { target: { value: `${plaintext.content}PORT=9090\n` } });
    expect(await screen.findByRole("alert")).toHaveTextContent("第 4 行");
    expect(screen.getByRole("button", { name: "保存 Env" })).toBeDisabled();
    expect(updateCalls).toBe(0);

    fireEvent.change(screen.getByLabelText("api.env 原文"), { target: { value: "# API\nPORT=9090\nTOKEN=new-secret\n" } });
    await user.click(screen.getByRole("button", { name: "保存 Env" }));
    const dialog = await screen.findByRole("dialog", { name: "保存 api.env？" });
    expect(within(dialog).getByText("~ PORT=••••••")).toBeInTheDocument();
    expect(within(dialog).getByText("~ TOKEN=••••••")).toBeInTheDocument();
    expect(dialog).toHaveTextContent("3 个目标节点");
    expect(dialog).not.toHaveTextContent("9090");
    expect(dialog).not.toHaveTextContent("new-secret");
  });

  it("版本冲突不会覆盖并要求重新加载", async () => {
    mockApplicationShell();
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", () => HttpResponse.json({ action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" })),
      http.get("/api/v1/application-env-files/env-1", () => HttpResponse.json(plaintext)),
      http.put("/api/v1/application-env-files/env-1", () => HttpResponse.json({ code: "version_conflict", message: "版本冲突", request_id: "req-conflict" }, { status: 409 })),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/env-1");
    await user.type(await screen.findByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    await user.clear(await screen.findByDisplayValue("8080"));
    await user.type(screen.getByLabelText("PORT 的值"), "9090");
    await user.click(screen.getByRole("button", { name: "保存 Env" }));
    await user.click(await screen.findByRole("button", { name: "确认保存" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("配置已被其他管理员更新");
    expect(screen.getByRole("button", { name: "重新加载最新版本" })).toBeInTheDocument();
  });

  it("保存成功后更新版本并在离开页面时移除明文", async () => {
    mockApplicationShell();
    let updated = false;
    let savedBody: unknown;
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", () => HttpResponse.json({ action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" })),
      http.get("/api/v1/application-env-files/env-1", () => HttpResponse.json(plaintext)),
      http.put("/api/v1/application-env-files/env-1", async ({ request }) => {
        savedBody = await request.json();
        updated = true;
        return HttpResponse.json({ ...plaintext, content: "# API\nPORT=9090\nTOKEN=top-secret\n", env_version: 4, version: 5 });
      }),
      http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [{ ...envFile, current_version: updated ? 4 : 3, version: updated ? 5 : 4 }] })),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/env-1");
    await user.type(await screen.findByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    await user.clear(await screen.findByDisplayValue("8080"));
    await user.type(screen.getByLabelText("PORT 的值"), "9090");
    await user.click(screen.getByRole("button", { name: "保存 Env" }));
    await user.click(await screen.findByRole("button", { name: "确认保存" }));
    expect(await screen.findByDisplayValue("9090")).toBeInTheDocument();
    expect(savedBody).toEqual({ content: "# API\nPORT=9090\nTOKEN=top-secret\n", expected_version: 4 });
    await user.click(screen.getByRole("link", { name: "返回应用" }));
    expect(await screen.findByText("Voucher Hub")).toBeInTheDocument();
    expect(screen.queryByText("top-secret")).not.toBeInTheDocument();
    expect(screen.queryByDisplayValue("9090")).not.toBeInTheDocument();
  });

  it("离开未保存的 Env 前要求确认且取消后保留草稿", async () => {
    mockApplicationShell();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    server.use(
      http.post("/api/v1/applications/app-1/env-reveal-grants", () => HttpResponse.json({ action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" })),
      http.get("/api/v1/application-env-files/env-1", () => HttpResponse.json(plaintext)),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config/env-1");
    await user.type(await screen.findByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    await user.clear(await screen.findByDisplayValue("8080"));
    await user.type(screen.getByLabelText("PORT 的值"), "9090");
    await user.click(screen.getByRole("link", { name: "返回应用" }));
    expect(confirm).toHaveBeenCalledWith("当前页面有未保存的修改，确定离开吗？");
    expect(screen.getByLabelText("PORT 的值")).toHaveValue("9090");
  });

  it("管理员重试失败同步并在删除前确认影响目标", async () => {
    mockApplicationShell();
    let retryCalls = 0;
    server.use(
      http.post("/api/v1/application-env-files/env-1/sync-retry", ({ request }) => {
        expect(new URL(request.url).searchParams.get("target_id")).toBe("target-failed");
        retryCalls += 1;
        return HttpResponse.json({ retried: 1 });
      }),
      http.post("/api/v1/applications/app-1/env-reveal-grants", async ({ request }) => {
        const body = await request.json() as { action: string };
        return HttpResponse.json({ action: body.action, grant_token: `grant-${body.action}`, expires_at: "2099-08-06T03:05:00Z" });
      }),
      http.get("/api/v1/application-env-files/env-1", () => HttpResponse.json(plaintext)),
      http.delete("/api/v1/application-env-files/env-1", () => new HttpResponse(null, { status: 204 })),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");
    expect(await screen.findByText("Node Success")).toBeInTheDocument();
    expect(screen.getByText("实际版本 v3")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重试 Node Failed 的 Env 同步" }));
    expect(retryCalls).toBe(1);
    expect(screen.getByRole("link", { name: "编辑 api.env" })).toHaveAttribute("href", "/apps/app-1/config/env-1");
    await user.click(screen.getByRole("link", { name: "编辑 api.env" }));
    await user.type(await screen.findByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并读取" }));
    await user.click(await screen.findByRole("button", { name: "删除 Env" }));
    await user.type(screen.getByLabelText("管理员密码"), "password");
    await user.click(screen.getByRole("button", { name: "验证并继续删除" }));
    const dialog = await screen.findByRole("dialog", { name: "删除 api.env？" });
    expect(dialog).toHaveTextContent("3 个目标节点");
    expect(dialog).toHaveTextContent("节点上的对应文件将被删除");
  });
});
