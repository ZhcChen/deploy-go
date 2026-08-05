import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = {
  status: "authenticated",
  csrfToken: "csrf-agent-node",
  user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" },
};
const node = {
  id: "node-1", name: "生产节点", host: null, port: null, username: null,
  ssh_credential_id: null, work_root: "/var/lib/deploy-go-agent/apps",
  secrets_root: "/var/lib/deploy-go-agent/secrets", status: "online",
  trusted_host_fingerprint: null, checked_at: null, version: 1,
  created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z",
};
const agent = { id: "agent-1", node_id: "node-1", name: "生产 Agent", environment: "prod", status: "online", agent_version: "0.1.0", hostname: "prod-01", architecture: "x86_64", created_at: "2026-08-01T00:00:00Z" };

function renderRoute(identity: "administrator" | "user" = "administrator") {
  return render(<MemoryRouter initialEntries={["/nodes/node-1"]}><AppProviders initialAuth={{ ...administrator, user: { ...administrator.user!, identity } }}><AppRoutes /></AppProviders></MemoryRouter>);
}

describe("Agent 节点管理", () => {
  it("在线 Agent 下发 SystemInspect 能力检查", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.post("/api/v1/nodes/node-1/checks", () => HttpResponse.json({ id: "check-result", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 10737418240, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, { status: 201 })),
    );
    const user = userEvent.setup();
    renderRoute();
    expect(await screen.findByRole("link", { name: "查看 Agent" })).toHaveAttribute("href", "/agents/agent-1");
    await user.click(screen.getByRole("button", { name: "执行检查" }));
    expect(await screen.findByText("10.0 GiB")).toBeInTheDocument();
  });

  it("离线 Agent 阻止能力检查", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, status: "offline" })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, status: "offline" }], next_cursor: null })),
    );
    renderRoute();
    expect(await screen.findByText("Agent 离线，恢复连接后才能执行检查。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "执行检查" })).toBeDisabled();
  });

  it("普通用户不加载或显示 Agent 管理", async () => {
    let agentCalls = 0;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => { agentCalls += 1; return HttpResponse.json({ items: [] }); }),
    );
    renderRoute("user");
    expect(await screen.findByRole("heading", { name: "生产节点" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "执行检查" })).not.toBeInTheDocument();
    expect(agentCalls).toBe(0);
  });
});
