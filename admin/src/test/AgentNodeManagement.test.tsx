import { act, render, screen, waitFor, within } from "@testing-library/react";
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
  trusted_host_fingerprint: null, checked_at: null, max_concurrent_deployments: 1, version: 1,
  created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z",
};
const agent = { id: "agent-1", node_id: "node-1", name: "生产 Agent", environment: "prod", status: "online", concurrent_deployments_enabled: false, agent_version: "0.3.4", hostname: "prod-01", architecture: "x86_64", created_at: "2026-08-01T00:00:00Z" };

function renderRoute(identity: "administrator" | "user" = "administrator", entry = "/nodes/node-1") {
  return render(<MemoryRouter initialEntries={[entry]}><AppProviders initialAuth={{ ...administrator, user: { ...administrator.user!, identity } }}><AppRoutes /><HistoryProbe /></AppProviders></MemoryRouter>);
}

function HistoryProbe() {
  const location = useLocation();
  const navigate = useNavigate();
  return <div style={{ position: "fixed", left: "-10000px" }}><output data-testid="location">{location.pathname}{location.search}</output><button type="button" onClick={() => navigate(-1)}>测试后退</button><button type="button" onClick={() => navigate(1)}>测试前进</button></div>;
}

describe("Agent 节点管理", () => {
  beforeEach(() => {
    server.use(http.get("/api/v1/nodes/:id/telemetry", () => HttpResponse.json({ node_id: "node-1", connectivity: "online", capability: "supported", freshness: "empty", captured_at: null, received_at: null, latest: null, history: [] })));
  });
  it("在线 Agent 下发 SystemInspect 能力检查", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.post("/api/v1/nodes/node-1/checks", () => HttpResponse.json({ id: "check-result", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 10737418240, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, { status: 201 })),
    );
    const user = userEvent.setup();
    renderRoute();
    await user.click(await screen.findByRole("tab", { name: "协同程序" }));
    expect(await screen.findByRole("heading", { name: "节点协同程序" })).toBeInTheDocument();
    expect(screen.getByText("v0.3.4")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "执行检查" }));
    expect(await screen.findByText("10.0 GiB")).toBeInTheDocument();
  });

  it("离线 Agent 阻止能力检查", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, status: "offline" })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, status: "offline" }], next_cursor: null })),
    );
    renderRoute("administrator", "/nodes/node-1?view=agent");
    expect(await screen.findByText("节点离线，恢复连接后才能执行检查。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "执行检查" })).toBeDisabled();
  });

  it("节点详情页在 WebSocket 通知之外有轮询兜底：状态变化无需刷新页面", async () => {
    // 只伪造 setInterval：TanStack Query 的轮询用它，测试库自己的等待逻辑不受影响。
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    try {
      let status = "online";
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, status })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      );
      renderRoute();
      expect((await screen.findAllByText("在线")).length).toBeGreaterThan(0);

      status = "offline";
      await act(async () => { vi.advanceTimersByTime(5_000); });

      expect((await screen.findAllByText("离线")).length).toBeGreaterThan(0);
    } finally {
      vi.useRealTimers();
    }
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
    expect(screen.queryByRole("tab", { name: "协同程序" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "生命周期" })).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByRole("button", { name: "重命名节点" })).not.toBeInTheDocument();
    expect(agentCalls).toBe(0);
  });

  it("管理员重命名节点后详情标题同步更新", async () => {
    let renamed = node;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(renamed)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.patch("/api/v1/nodes/node-1", async ({ request }) => {
        const body = (await request.json()) as { name: string };
        renamed = { ...node, name: body.name, version: 2 };
        return HttpResponse.json(renamed);
      }),
    );
    const user = userEvent.setup();
    renderRoute();
    expect(await screen.findByRole("heading", { name: "生产节点" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重命名节点" }));
    const input = screen.getByLabelText("节点名称");
    await user.clear(input);
    await user.type(input, "测试节点 01");
    await user.click(screen.getByRole("button", { name: "保存名称" }));
    expect(await screen.findByRole("heading", { name: "测试节点 01" })).toBeInTheDocument();
  });

  describe("同节点并发部署", () => {
    it("本机未开启时说明设置暂不生效，实际并发为 1", async () => {
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, max_concurrent_deployments: 10 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, concurrent_deployments_enabled: false }], next_cursor: null })),
      );
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      expect(within(section).getByText("并发部署数").nextElementSibling).toHaveTextContent("10");
      expect(await within(section).findByText("未开启")).toBeInTheDocument();
      expect(within(section).getByText("当前实际并发").nextElementSibling).toHaveTextContent("1");
      expect(within(section).getByText(/本机没有开启并发，设置的并发部署数暂不生效/)).toBeInTheDocument();
    });

    it("本机已开启时实际并发等于设置值，且不显示未生效提示", async () => {
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, max_concurrent_deployments: 10 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, concurrent_deployments_enabled: true }], next_cursor: null })),
      );
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      expect(await within(section).findByText("已开启")).toBeInTheDocument();
      expect(within(section).getByText("当前实际并发").nextElementSibling).toHaveTextContent("10");
      expect(within(section).queryByText(/暂不生效/)).not.toBeInTheDocument();
    });

    it("管理员修改并发部署数时携带 CSRF 与节点版本，保存后显示新值", async () => {
      let received: { headers: Headers; body: unknown } | null = null;
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, version: 7, max_concurrent_deployments: 1 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, concurrent_deployments_enabled: true }], next_cursor: null })),
        http.put("/api/v1/nodes/node-1/deployment-concurrency", async ({ request }) => {
          received = { headers: request.headers, body: await request.json() };
          return HttpResponse.json({ ...node, version: 8, max_concurrent_deployments: 5 });
        }),
      );
      const user = userEvent.setup();
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      await user.click(within(section).getByRole("button", { name: "修改并发部署数" }));
      const input = within(section).getByLabelText(/^并发部署数/);
      await user.clear(input);
      await user.type(input, "5");
      await user.click(within(section).getByRole("button", { name: "保存" }));
      await waitFor(() => expect(received).not.toBeNull());
      expect(received!.headers.get("x-csrf-token")).toBe("csrf-agent-node");
      expect(received!.body).toEqual({ max_concurrent_deployments: 5, version: 7 });
      await waitFor(() => expect(within(section).getByText("并发部署数").nextElementSibling).toHaveTextContent("5"));
      expect(within(section).queryByLabelText(/^并发部署数/)).not.toBeInTheDocument();
    });

    it("超出 1 至 16 或与当前值相同时不能保存", async () => {
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, max_concurrent_deployments: 3 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, concurrent_deployments_enabled: true }], next_cursor: null })),
      );
      const user = userEvent.setup();
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      await user.click(within(section).getByRole("button", { name: "修改并发部署数" }));
      const input = within(section).getByLabelText(/^并发部署数/);
      const save = within(section).getByRole("button", { name: "保存" });
      expect(save).toBeDisabled(); // 与当前值相同
      for (const invalid of ["0", "17", "-1", "1.5"]) {
        await user.clear(input);
        await user.type(input, invalid);
        expect(save).toBeDisabled();
      }
      await user.clear(input);
      await user.type(input, "16");
      expect(save).toBeEnabled();
    });

    it("保存失败时显示错误并保留编辑状态", async () => {
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, max_concurrent_deployments: 1 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
        http.put("/api/v1/nodes/node-1/deployment-concurrency", () => HttpResponse.json({ code: "resource_version_conflict", message: "节点已经被其他请求修改，请刷新后重试", request_id: "req_1" }, { status: 409 })),
      );
      const user = userEvent.setup();
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      await user.click(within(section).getByRole("button", { name: "修改并发部署数" }));
      const input = within(section).getByLabelText(/^并发部署数/);
      await user.clear(input);
      await user.type(input, "4");
      await user.click(within(section).getByRole("button", { name: "保存" }));
      expect(await within(section).findByText(/已经被其他请求修改/)).toBeInTheDocument();
      expect(within(section).getByLabelText(/^并发部署数/)).toHaveValue(4);
    });

    it("普通用户只读：看到数量但没有修改入口，也不加载 Agent 信息", async () => {
      let agentsRequested = false;
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, max_concurrent_deployments: 10 })),
        http.get("/api/v1/agents", () => { agentsRequested = true; return HttpResponse.json({ items: [agent], next_cursor: null }); }),
      );
      renderRoute("user");
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      expect(within(section).getByText("并发部署数").nextElementSibling).toHaveTextContent("10");
      expect(within(section).queryByRole("button", { name: "修改并发部署数" })).not.toBeInTheDocument();
      expect(within(section).queryByText("本机并发")).not.toBeInTheDocument();
      expect(agentsRequested).toBe(false);
    });

    it("已归档节点不提供修改入口", async () => {
      server.use(
        http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, archived_at: "2026-08-02T00:00:00Z", max_concurrent_deployments: 2 })),
        http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      );
      renderRoute();
      const section = (await screen.findByRole("heading", { name: "同节点并发部署" })).closest("section")!;
      expect(within(section).queryByRole("button", { name: "修改并发部署数" })).not.toBeInTheDocument();
    });
  });

  it("节点详情默认显示概览并支持 SSH 深链", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", available: true, unavailable_code: null,
        agent_id: "agent-1", agent_online: true, identity_valid: true, protocol_version: 11,
        pty_terminal: true,
      })),
    );
    const user = userEvent.setup();
    renderRoute();
    expect(await screen.findByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByRole("heading", { name: "节点协同程序" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "SSH" }));
    expect(await screen.findByRole("tabpanel", { name: "SSH" })).toBeVisible();
    expect(await screen.findByRole("button", { name: "连接终端" })).toBeEnabled();
    expect(screen.queryByRole("switch", { name: "启用特权执行" })).not.toBeInTheDocument();
    expect(screen.getByTestId("location")).toHaveTextContent("/nodes/node-1?view=ssh");
    await user.click(screen.getByText("测试后退"));
    expect(await screen.findByRole("tab", { name: "概览" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByText("测试前进"));
    expect(await screen.findByRole("tab", { name: "SSH" })).toHaveAttribute("aria-selected", "true");
  });

  it("管理员通过 view=ssh 刷新后保留 executor 不可用提示", async () => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", available: false,
        unavailable_code: "terminal_executor_unavailable", agent_id: "agent-1",
        agent_online: true, identity_valid: true, protocol_version: 11, pty_terminal: false,
      })),
    );
    renderRoute("administrator", "/nodes/node-1?view=ssh");
    expect(await screen.findByRole("tab", { name: "SSH" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("节点终端 executor 不可用")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "连接终端" })).not.toBeInTheDocument();
  });

  it.each([
    ["terminal_agent_identity_invalid", "节点 Agent 身份无效或已撤销"],
    ["terminal_agent_offline", "节点 Agent 当前离线"],
    ["terminal_protocol_unsupported", "Agent 版本不支持终端"],
    ["terminal_executor_unavailable", "节点终端 executor 不可用"],
  ])("SSH 视图准确显示门禁 %s", async (code, message) => {
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes/node-1/terminal-capability", () => HttpResponse.json({
        node_id: "node-1", available: false, unavailable_code: code,
        agent_id: "agent-1", agent_online: code !== "terminal_agent_offline",
        identity_valid: code !== "terminal_agent_identity_invalid",
        protocol_version: code === "terminal_protocol_unsupported" ? 10 : 11,
        pty_terminal: code !== "terminal_executor_unavailable",
      })),
    );
    renderRoute("administrator", "/nodes/node-1?view=ssh");
    expect(await screen.findByRole("heading", { name: message })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "连接终端" })).not.toBeInTheDocument();
  });

  it("管理员归档节点后显示已归档标识并可恢复", async () => {
    let archived = false;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json({ ...node, archived_at: archived ? "2026-08-03T00:00:00Z" : null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.post("/api/v1/nodes/node-1/archive", () => {
        archived = true;
        return new HttpResponse(null, { status: 204 });
      }),
      http.post("/api/v1/nodes/node-1/unarchive", () => {
        archived = false;
        return new HttpResponse(null, { status: 204 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("administrator", "/nodes/node-1?view=lifecycle");
    expect(await screen.findByRole("tab", { name: "生命周期" })).toHaveAttribute("aria-selected", "true");
    const archiveButton = await screen.findByRole("button", { name: "归档节点" });
    await user.click(archiveButton);
    expect(await screen.findByRole("heading", { name: /归档 生产节点 节点/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "确认归档" }));
    expect(await screen.findByText("已归档", { selector: ".status-badge--archived" })).toBeInTheDocument();
    const restoreButton = await screen.findByRole("button", { name: "恢复节点" });
    await user.click(restoreButton);
    await user.click(screen.getByRole("button", { name: "确认恢复" }));
    expect(await screen.findByRole("button", { name: "归档节点" })).toBeInTheDocument();
  });

  it("节点列表支持正常与已归档过滤", async () => {
    const activeNode = { ...node, id: "node-active", name: "正常节点" };
    server.use(
      http.get("/api/v1/nodes", ({ request }) => {
        const archived = new URL(request.url).searchParams.get("archived") === "true";
        return HttpResponse.json({
          items: archived ? [{ ...node, id: "node-archived", name: "已归档节点", archived_at: "2026-08-03T00:00:00Z" }] : [activeNode],
          next_cursor: null,
        });
      }),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [{ ...agent, node_id: "node-active", environment: "test" }, { ...agent, id: "agent-archived", node_id: "node-archived", environment: "test" }], next_cursor: null })),
    );
    const user = userEvent.setup();
    renderRoute("administrator", "/nodes");
    expect(await screen.findByRole("link", { name: "管理节点 正常节点" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "节点状态" }));
    await user.click(await screen.findByRole("option", { name: "已归档" }));
    expect(await screen.findByRole("link", { name: "管理节点 已归档节点" })).toBeInTheDocument();
    expect(screen.getByText("已归档", { selector: ".node-card__status--archived" })).toBeInTheDocument();
  });

});
