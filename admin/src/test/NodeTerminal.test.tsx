import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { vi } from "vitest";
import { NodeTerminal } from "../features/nodes/NodeTerminal";
import { server } from "./server";

const terminalDoubles = vi.hoisted(() => ({
  instances: [] as Array<{
    write: ReturnType<typeof vi.fn>;
    writeln: ReturnType<typeof vi.fn>;
    clear: ReturnType<typeof vi.fn>;
    dispose: ReturnType<typeof vi.fn>;
    data?: (value: string) => void;
    resize?: (size: { cols: number; rows: number }) => void;
  }>,
  fits: [] as Array<ReturnType<typeof vi.fn>>,
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 100;
    rows = 30;
    write = vi.fn();
    writeln = vi.fn();
    clear = vi.fn();
    dispose = vi.fn();
    loadAddon = vi.fn();
    open = vi.fn();
    focus = vi.fn();
    data?: (value: string) => void;
    resize?: (size: { cols: number; rows: number }) => void;
    constructor() { terminalDoubles.instances.push(this); }
    onData(callback: (value: string) => void) { this.data = callback; return { dispose: vi.fn() }; }
    onResize(callback: (size: { cols: number; rows: number }) => void) { this.resize = callback; return { dispose: vi.fn() }; }
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = vi.fn();
    constructor() { terminalDoubles.fits.push(this.fit); }
  },
}));

class WebSocketDouble {
  static instances: WebSocketDouble[] = [];
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  readonly url: string;
  readonly protocols: string[];
  readyState = WebSocketDouble.CONNECTING;
  sent: string[] = [];
  close = vi.fn(() => { this.readyState = WebSocketDouble.CLOSED; });
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  constructor(url: string | URL, protocols?: string | string[]) {
    this.url = String(url);
    this.protocols = typeof protocols === "string" ? [protocols] : protocols ?? [];
    WebSocketDouble.instances.push(this);
  }
  send(value: string) { this.sent.push(value); }
  open() { this.readyState = WebSocketDouble.OPEN; this.onopen?.(new Event("open")); }
  message(value: object) { this.onmessage?.(new MessageEvent("message", { data: JSON.stringify(value) })); }
  fail() { this.onerror?.(new Event("error")); }
  finishClose() { this.readyState = WebSocketDouble.CLOSED; this.onclose?.(new CloseEvent("close")); }
}

const available = {
  nodeId: "node-1", privilegedExecution: true, available: true, unavailableCode: null,
  agentId: "agent-1", agentOnline: true, identityValid: true, protocolVersion: 5,
  ptyTerminal: true,
};

describe("节点 SSH 终端", () => {
  beforeEach(() => {
    terminalDoubles.instances.length = 0;
    terminalDoubles.fits.length = 0;
    WebSocketDouble.instances.length = 0;
    vi.stubGlobal("WebSocket", WebSocketDouble);
    server.use(http.post("/api/v1/nodes/node-1/terminal-sessions", ({ request }) => {
      expect(request.headers.get("X-CSRF-Token")).toBe("csrf-terminal");
      return HttpResponse.json({ id: "term-1", node_id: "node-1", agent_id: "agent-1", actor_id: "admin-1", status: "opening", started_at: "2026-08-07T00:00:00Z", input_bytes: 0, output_bytes: 0 }, { status: 201 });
    }));
  });

  it("连接后转发 open、输入、resize、Ctrl+C 和主动关闭", async () => {
    const user = userEvent.setup();
    render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    await waitFor(() => expect(WebSocketDouble.instances).toHaveLength(1));
    const socket = WebSocketDouble.instances[0];
    expect(socket.url).toContain("/api/v1/terminal-sessions/term-1/stream");
    expect(socket.protocols).toEqual(["deploy-go-terminal.v1", "csrf.csrf-terminal"]);
    act(() => socket.open());
    expect(JSON.parse(socket.sent[0])).toEqual({ type: "open", columns: 100, rows: 30 });
    act(() => socket.message({ type: "opened", session_id: "term-1", sequence: 1 }));
    expect(await screen.findByText("已连接")).toBeInTheDocument();

    act(() => terminalDoubles.instances[0].data?.("whoami\r"));
    expect(JSON.parse(socket.sent[1])).toMatchObject({ type: "input", sequence: 1, encoding: "base64", data: "d2hvYW1pDQ==" });
    act(() => terminalDoubles.instances[0].resize?.({ cols: 120, rows: 40 }));
    expect(JSON.parse(socket.sent[2])).toEqual({ type: "resize", sequence: 2, columns: 120, rows: 40 });
    await user.click(screen.getByRole("button", { name: "发送 Ctrl+C" }));
    expect(JSON.parse(socket.sent[3])).toMatchObject({ type: "input", sequence: 3, data: "Aw==" });
    await user.click(screen.getByRole("button", { name: "关闭终端" }));
    expect(JSON.parse(socket.sent[4])).toEqual({ type: "close", sequence: 4 });
  });

  it("解码输出并提供清屏操作", async () => {
    const user = userEvent.setup();
    render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const socket = WebSocketDouble.instances[0];
    act(() => { socket.open(); socket.message({ type: "opened", session_id: "term-1", sequence: 1 }); });
    act(() => socket.message({ type: "output", session_id: "term-1", sequence: 2, encoding: "base64", data: "cm9vdEBwcm9kLTAxOi9zcnYjIA==" }));
    expect(terminalDoubles.instances[0].write).toHaveBeenCalledWith(expect.any(Uint8Array));
    await user.click(screen.getByRole("button", { name: "清空终端" }));
    expect(terminalDoubles.instances[0].clear).toHaveBeenCalledTimes(1);
  });

  it("大段输入按协议上限拆帧并保持连续序号", async () => {
    const user = userEvent.setup();
    render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const socket = WebSocketDouble.instances[0];
    act(() => { socket.open(); socket.message({ type: "opened", session_id: "term-1", sequence: 1 }); });
    act(() => terminalDoubles.instances[0].data?.("x".repeat(70 * 1024)));
    const frames = socket.sent.slice(1).map((value) => JSON.parse(value) as { sequence: number; data: string });
    expect(frames).toHaveLength(2);
    expect(frames.map((frame) => frame.sequence)).toEqual([1, 2]);
    expect(frames.every((frame) => frame.data.length <= 87_384)).toBe(true);
  });

  it("异常断线显示错误且卸载时关闭 WebSocket", async () => {
    const user = userEvent.setup();
    const rendered = render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const socket = WebSocketDouble.instances[0];
    act(() => { socket.open(); socket.fail(); });
    expect(await screen.findByText("终端连接异常中断")).toBeInTheDocument();
    rendered.unmount();
    expect(socket.close).toHaveBeenCalled();
    expect(terminalDoubles.instances[0].dispose).toHaveBeenCalled();
  });

  it("拒绝错误会话序号并在主动关闭断线时收敛状态", async () => {
    const user = userEvent.setup();
    render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const socket = WebSocketDouble.instances[0];
    act(() => { socket.open(); socket.message({ type: "opened", session_id: "term-other", sequence: 1 }); });
    expect(await screen.findByText("终端返回了无效的会话序号")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const nextSocket = WebSocketDouble.instances[1];
    act(() => { nextSocket.open(); nextSocket.message({ type: "opened", session_id: "term-1", sequence: 1 }); });
    await user.click(screen.getByRole("button", { name: "关闭终端" }));
    act(() => nextSocket.finishClose());
    expect(await screen.findByText("已关闭")).toBeInTheDocument();
  });

  it("不会把终端正文写入浏览器持久化存储", async () => {
    const local = vi.spyOn(Storage.prototype, "setItem");
    const user = userEvent.setup();
    render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    await user.click(screen.getByRole("button", { name: "连接终端" }));
    const socket = WebSocketDouble.instances[0];
    act(() => { socket.open(); socket.message({ type: "opened", session_id: "term-1", sequence: 1 }); terminalDoubles.instances[0].data?.("SECRET_TOKEN=value\r"); });
    expect(local).not.toHaveBeenCalled();
  });

  it("会话创建期间卸载会主动收敛已创建会话", async () => {
    let releaseCreate!: () => void;
    const createGate = new Promise<void>((resolve) => { releaseCreate = resolve; });
    let closed = false;
    server.use(
      http.post("/api/v1/nodes/node-1/terminal-sessions", async () => {
        await createGate;
        return HttpResponse.json({ id: "term-race", node_id: "node-1", agent_id: "agent-1", status: "opening" }, { status: 201 });
      }),
      http.post("/api/v1/terminal-sessions/term-race/close", ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-terminal");
        closed = true;
        return HttpResponse.json({ id: "term-race", node_id: "node-1", agent_id: "agent-1", status: "closed" });
      }),
    );
    const rendered = render(<NodeTerminal nodeId="node-1" nodeName="生产节点" csrfToken="csrf-terminal" capability={available} />);
    fireEvent.click(screen.getByRole("button", { name: "连接终端" }));
    await waitFor(() => expect(terminalDoubles.instances).toHaveLength(1));
    rendered.unmount();
    releaseCreate();
    await waitFor(() => expect(closed).toBe(true));
    expect(WebSocketDouble.instances).toHaveLength(0);
  });
});
