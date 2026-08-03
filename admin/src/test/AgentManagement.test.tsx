import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-agent", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const agent = { id: "agent-1", node_id: "node-1", name: "生产节点 01", status: "offline", registered_at: null, last_seen_at: null, agent_version: null, hostname: null, architecture: null, revoked_at: null, created_at: "2026-08-03T00:00:00Z" };
const command = "printf '%s\\n' 'dga_enroll_fixture' | sudo bash";

function renderRoute(path: string, snapshot = administrator) {
  return render(<MemoryRouter initialEntries={[path]}><AppProviders initialAuth={snapshot}><AppRoutes /></AppProviders></MemoryRouter>);
}

describe("Agent 管理", () => {
  it("创建后立即显示离线 Agent 和一次性安装命令", async () => {
    let body: unknown;
    server.use(
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.post("/api/v1/agents", async ({ request }) => { expect(request.headers.get("X-CSRF-Token")).toBe("csrf-agent"); body = await request.json(); return HttpResponse.json({ agent, enrollment_token: "dga_enroll_fixture", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: command }, { status: 201 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/agents");
    await user.click(screen.getByRole("button", { name: "创建 Agent" }));
    await user.type(screen.getByLabelText("Agent 名称"), "生产节点 01");
    await user.click(screen.getByRole("button", { name: "创建并生成命令" }));
    await waitFor(() => expect(body).toEqual({ name: "生产节点 01" }));
    expect(screen.getByText(command)).toBeInTheDocument();
    expect(screen.getByText(/当前离线/)).toBeInTheDocument();
  });

  it("重新生成与撤销均经过明确确认", async () => {
    let regenerated = 0;
    let revoked = 0;
    server.use(
      http.get("/api/v1/agents/agent-1", () => HttpResponse.json(agent)),
      http.post("/api/v1/agents/agent-1/install-command", () => { regenerated += 1; return HttpResponse.json({ agent_id: "agent-1", enrollment_token: "new-token", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: command }); }),
      http.post("/api/v1/agents/agent-1/revoke", () => { revoked += 1; return new HttpResponse(null, { status: 204 }); }),
    );
    const user = userEvent.setup();
    renderRoute("/agents/agent-1");
    await user.click(await screen.findByRole("button", { name: "重新生成命令" }));
    expect(regenerated).toBe(0);
    await user.click(screen.getByRole("button", { name: "确认重新生成" }));
    await waitFor(() => expect(regenerated).toBe(1));
    expect(screen.getByText(command)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "撤销 Agent" }));
    expect(revoked).toBe(0);
    await user.click(screen.getByRole("button", { name: "确认撤销" }));
    await waitFor(() => expect(revoked).toBe(1));
    expect(screen.queryByText(command)).not.toBeInTheDocument();
  });

  it("普通用户没有 Agent 导航且深链返回 403", () => {
    renderRoute("/agents", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(screen.queryByRole("link", { name: "Agent" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
  });
});
