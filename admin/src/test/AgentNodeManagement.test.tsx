import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useLocation, useNavigate } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { vi } from "vitest";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

vi.mock("../features/nodes/NodeTerminal", () => ({
  NodeTerminal: () => <button type="button">连接终端</button>,
}));

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

function renderRoute(identity: "administrator" | "user" = "administrator", entry = "/nodes/node-1") {
  return render(<MemoryRouter initialEntries={[entry]}><AppProviders initialAuth={{ ...administrator, user: { ...administrator.user!, identity } }}><AppRoutes /><HistoryProbe /></AppProviders></MemoryRouter>);
}

function HistoryProbe() {
  const location = useLocation();
  const navigate = useNavigate();
  return <div style={{ position: "fixed", left: "-10000px" }}><output data-testid="location">{location.pathname}{location.search}</output><button type="button" onClick={() => navigate(-1)}>测试后退</button><button type="button" onClick={() => navigate(1)}>测试前进</button></div>;
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
    expect(await screen.findByRole("heading", { name: "节点协同程序" })).toBeInTheDocument();
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "执行检查" }));
    expect(await screen.findByText("10.0 GiB")).toBeInTheDocument();
  });

  it("离线 Agent 阻止能力检查", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, status: "offline" })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, status: "offline" }], next_cursor: null })),
    );
    renderRoute();
    expect(await screen.findByText("节点离线，恢复连接后才能执行检查。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "执行检查" })).toBeDisabled();
  });

  it("普通用户不加载或显示 Agent 管理", async () => {
    let agentCalls = 0;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => { agentCalls += 1; return HttpResponse.json({ items: [] }); }),
    );
    renderRoute("user", "/nodes/node-1?view=ssh");
    expect(await screen.findByRole("heading", { name: "生产节点" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "执行检查" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "SSH" })).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    expect(agentCalls).toBe(0);
  });

  it("节点详情默认显示概览并支持 SSH 深链", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, privileged_execution: true })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", privileged_execution: true, available: true, unavailable_code: null,
        agent_id: "agent-1", agent_online: true, identity_valid: true, protocol_version: 6,
        pty_terminal: true,
      })),
    );
    const user = userEvent.setup();
    renderRoute();
    expect(await screen.findByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByRole("tab", { name: "SSH" }));
    expect(await screen.findByRole("tabpanel", { name: "SSH" })).toBeVisible();
    expect(await screen.findByRole("button", { name: "连接终端" })).toBeEnabled();
    expect(screen.getByTestId("location")).toHaveTextContent("/nodes/node-1?view=ssh");
    await user.click(screen.getByText("测试后退"));
    expect(await screen.findByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByText("测试前进"));
    expect(await screen.findByRole("tab", { name: "SSH" })).toHaveAttribute("aria-selected", "true");
  });

  it("管理员通过 view=ssh 刷新后仍显示 SSH 视图", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, privileged_execution: false })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", privileged_execution: false, available: false,
        unavailable_code: "terminal_privileged_execution_disabled", agent_id: "agent-1",
        agent_online: true, identity_valid: true, protocol_version: 6, pty_terminal: true,
      })),
    );
    renderRoute("administrator", "/nodes/node-1?view=ssh");
    expect(await screen.findByRole("tab", { name: "SSH" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("节点尚未启用特权执行")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "连接终端" })).not.toBeInTheDocument();
  });

  it.each([
    ["terminal_agent_identity_invalid", "节点 Agent 身份无效或已撤销"],
    ["terminal_agent_offline", "节点 Agent 当前离线"],
    ["terminal_protocol_unsupported", "Agent 版本不支持终端"],
    ["terminal_executor_unavailable", "节点终端 executor 不可用"],
  ])("SSH 视图准确显示门禁 %s", async (code, message) => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, privileged_execution: true })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", privileged_execution: true, available: false, unavailable_code: code,
        agent_id: "agent-1", agent_online: code !== "terminal_agent_offline",
        identity_valid: code !== "terminal_agent_identity_invalid",
        protocol_version: code === "terminal_protocol_unsupported" ? 5 : 6,
        pty_terminal: code !== "terminal_executor_unavailable",
      })),
    );
    renderRoute("administrator", "/nodes/node-1?view=ssh");
    expect(await screen.findByRole("heading", { name: message })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "连接终端" })).not.toBeInTheDocument();
  });

  it("管理员可在概览显式启用节点特权执行", async () => {
    let enabled = false;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, privileged_execution: enabled })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", privileged_execution: enabled, available: enabled,
        unavailable_code: enabled ? null : "terminal_privileged_execution_disabled", agent_id: "agent-1",
        agent_online: true, identity_valid: true, protocol_version: 6, pty_terminal: true,
      })),
      http.put("/api/v1/nodes/node-1/privileged-execution", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-agent-node");
        expect(await request.json()).toEqual({ enabled: true });
        enabled = true;
        return HttpResponse.json({ node_id: "node-1", enabled: true });
      }),
    );
    const user = userEvent.setup();
    renderRoute();
    const toggle = await screen.findByRole("switch", { name: "启用特权执行" });
    expect(toggle).not.toBeChecked();
    await user.click(toggle);
    await waitFor(() => expect(toggle).toBeChecked());
  });
});
