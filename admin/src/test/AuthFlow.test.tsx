import { http, HttpResponse } from "msw";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

function renderApp(path: string, state?: unknown) {
  return render(
    <MemoryRouter initialEntries={[state === undefined ? path : { pathname: path, state }]}>
      <AppProviders><AppRoutes /></AppProviders>
    </MemoryRouter>,
  );
}

function setupComplete() {
  server.use(
    http.get("/api/v1/setup", () => HttpResponse.json({ setup_required: false, setup_enabled: false })),
    http.get("/api/v1/auth/me", () => HttpResponse.json({ code: "not_authenticated", message: "未登录", request_id: "req-auth" }, { status: 401 })),
  );
}

describe("Web 认证流程", () => {
  it("空库未启用 setup 时阻止显示初始化表单", async () => {
    server.use(
      http.get("/api/v1/setup", () => HttpResponse.json({ setup_required: true, setup_enabled: false })),
    );
    renderApp("/setup");
    expect(await screen.findByRole("heading", { name: "初始化未启用" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Setup Token")).not.toBeInTheDocument();
  });

  it("未登录深链跳转登录并在成功后返回原站内页面", async () => {
    setupComplete();
    let loginCount = 0;
    server.use(
      http.post("/api/v1/auth/login", async ({ request }) => {
        loginCount += 1;
        const body = await request.json() as { username: string; password: string };
        expect(body).toEqual({ username: "admin", password: "password123" });
        return HttpResponse.json({
          csrf_token: "csrf-login",
          user: { id: "admin-1", username: "admin", display_name: "陈舟", identity: "administrator" },
        });
      }),
    );
    const user = userEvent.setup();
    renderApp("/nodes?status=offline");
    await screen.findByRole("heading", { name: "登录" });
    await user.type(screen.getByLabelText("账号或邮箱"), "admin");
    await user.type(screen.getByLabelText("密码"), "password123");
    await user.dblClick(screen.getByRole("button", { name: "登录" }));
    await screen.findByRole("heading", { level: 1, name: "节点" });
    expect(loginCount).toBe(1);
  });

  it("拒绝把外部地址作为登录返回路径", async () => {
    setupComplete();
    server.use(
      http.post("/api/v1/auth/login", () => HttpResponse.json({
        csrf_token: "csrf-login",
        user: { id: "admin-1", username: "admin", display_name: "陈舟", identity: "administrator" },
      })),
    );
    const user = userEvent.setup();
    renderApp("/login", { from: "/\\outside.invalid/path" });
    await screen.findByRole("heading", { name: "登录" });
    await user.type(screen.getByLabelText("账号或邮箱"), "admin");
    await user.type(screen.getByLabelText("密码"), "password123");
    await user.click(screen.getByRole("button", { name: "登录" }));
    await screen.findByRole("heading", { level: 1, name: "概览" });
  });

  it("setup token 只发送一次且不进入持久化存储", async () => {
    const persistedBefore = { ...localStorage };
    server.use(
      http.get("/api/v1/setup", () => HttpResponse.json({ setup_required: true, setup_enabled: true })),
      http.post("/api/v1/setup", async ({ request }) => {
        expect(request.headers.get("X-Setup-Token")).toBe("one-time-token");
        return HttpResponse.json({ id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    renderApp("/setup");
    await screen.findByRole("heading", { name: "创建管理员" });
    await user.type(screen.getByLabelText("Setup Token"), "one-time-token");
    await user.type(screen.getByLabelText("登录账号"), "admin");
    await user.type(screen.getByLabelText("初始密码"), "password123");
    await user.click(screen.getByRole("button", { name: "完成初始化" }));
    await screen.findByRole("heading", { name: "登录" });
    expect({ ...localStorage }).toEqual(persistedBefore);
    expect(document.body.textContent).not.toContain("one-time-token");
  });

  it("登录失败展示服务端消息但不回显密码", async () => {
    setupComplete();
    server.use(
      http.post("/api/v1/auth/login", () => HttpResponse.json({ code: "invalid_credentials", message: "凭据无效", request_id: "req-login" }, { status: 401 })),
    );
    const user = userEvent.setup();
    renderApp("/login");
    await screen.findByRole("heading", { name: "登录" });
    await user.type(screen.getByLabelText("账号或邮箱"), "admin");
    await user.type(screen.getByLabelText("密码"), "secret-password");
    await user.click(screen.getByRole("button", { name: "登录" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("账号或密码不正确");
    expect(screen.getByRole("alert")).toHaveTextContent("req-login");
    await waitFor(() => expect(document.body.textContent).not.toContain("secret-password"));
  });
});
