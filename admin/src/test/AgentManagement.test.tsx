import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-agent", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const agent = { id: "agent-1", node_id: "node-1", name: "生产节点 01", environment: "prod", status: "offline", registered_at: null, last_seen_at: null, agent_version: null, hostname: null, architecture: null, revoked_at: null, created_at: "2026-08-03T00:00:00Z" };
const command = "sudo env 'DEPLOY_GO_AGENT_ID=agent-1' 'DEPLOY_GO_AGENT_ENROLLMENT_TOKEN=dga_enroll_fixture' bash";

function renderRoute(path: string, snapshot = administrator) {
  return render(<MemoryRouter initialEntries={[path]}><AppProviders initialAuth={snapshot}><AppRoutes /></AppProviders></MemoryRouter>);
}

describe("节点协同程序管理", () => {
  it("创建节点后立即显示一次性安装命令", async () => {
    let body: unknown;
    server.use(
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.post("/api/v1/agents", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-agent"); body = await request.json(); return HttpResponse.json({ agent, enrollment_token: "dga_enroll_fixture", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: command }, { status: 201 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/nodes");
    await user.click(screen.getByRole("button", { name: "创建节点" }));
    await user.type(screen.getByLabelText("节点名称"), "生产节点 01");
    await user.click(screen.getByLabelText("环境"));
    await user.click(await screen.findByRole("option", { name: "生产环境" }));
    await user.click(screen.getByRole("button", { name: "创建并生成安装命令" }));
    await waitFor(() => expect(body).toEqual({ name: "生产节点 01", environment: "prod" }));
    expect(screen.getByText(command)).toBeInTheDocument();
    expect(screen.getByText(/目标 Linux 服务器/)).toBeInTheDocument();
  });

  it("可将 Agent 接管到未关联的历史节点", async () => {
    let body: unknown;
    server.use(
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes/node-legacy", () => HttpResponse.json({ id: "node-legacy", name: "历史节点", status: "offline", work_root: "/srv/apps", secrets_root: "/srv/secrets", version: 1 })),
      http.post("/api/v1/agents", async ({ request }) => { body = await request.json(); return HttpResponse.json({ agent: { ...agent, node_id: "node-legacy", name: "历史节点" }, enrollment_token: "dga_enroll_fixture", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: command }, { status: 201 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/nodes/node-legacy");
    await user.click(await screen.findByRole("button", { name: "安装协同程序" }));
    expect(screen.getByLabelText("节点名称")).toHaveValue("历史节点");
    await user.click(screen.getByLabelText("环境"));
    await user.click(await screen.findByRole("option", { name: "测试环境" }));
    await user.click(screen.getByRole("button", { name: "生成安装命令" }));
    await waitFor(() => expect(body).toEqual({ name: "历史节点", node_id: "node-legacy", environment: "test" }));
  });

  it("重新生成与撤销均经过明确确认", async () => {
    let regenerated = 0;
    let revoked = 0;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ id: "node-1", name: "生产节点 01", status: "offline", work_root: "/srv/apps", secrets_root: "/srv/secrets", version: 1 })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.post("/api/v1/agents/agent-1/install-command", () => { regenerated += 1; return HttpResponse.json({ agent_id: "agent-1", enrollment_token: "new-token", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: command }); }),
      http.post("/api/v1/agents/agent-1/revoke", () => { revoked += 1; return new HttpResponse(null, { status: 204 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/nodes/node-1");
    expect(await screen.findByRole("heading", { name: "节点协同程序" })).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "重新生成安装命令" }));
    expect(regenerated).toBe(0);
    await user.click(screen.getByRole("button", { name: "确认重新生成" }));
    await waitFor(() => expect(regenerated).toBe(1));
    expect(screen.getByText(command)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "撤销节点身份" }));
    expect(revoked).toBe(0);
    await user.click(screen.getByRole("button", { name: "确认撤销" }));
    await waitFor(() => expect(revoked).toBe(1));
    expect(screen.queryByText(command)).not.toBeInTheDocument();
  });

  it("普通用户没有 Agent 导航且旧管理深链返回 403", () => {
    renderRoute("/agents", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(screen.queryByRole("link", { name: "Agent" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
  });
});
