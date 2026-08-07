import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { Eraser, Keyboard, Link2, Link2Off, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { terminalApi, type TerminalCapability } from "./api";
import { openTerminalSocket, type TerminalSocket } from "./terminalSocket";

type ConnectionState = "disconnected" | "connecting" | "connected" | "closing" | "closed" | "error";

const stateLabels: Record<ConnectionState, string> = {
  disconnected: "未连接",
  connecting: "连接中",
  connected: "已连接",
  closing: "正在关闭",
  closed: "已关闭",
  error: "连接异常",
};

export function NodeTerminal({ nodeId, nodeName, csrfToken, capability }: {
  nodeId: string;
  nodeName: string;
  csrfToken: string;
  capability: TerminalCapability;
}) {
  const mountRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const socketRef = useRef<TerminalSocket | null>(null);
  const mountedRef = useRef(true);
  const inputDisposableRef = useRef<{ dispose(): void } | null>(null);
  const resizeDisposableRef = useRef<{ dispose(): void } | null>(null);
  const fitListenerRef = useRef<(() => void) | null>(null);
  const [state, setState] = useState<ConnectionState>("disconnected");
  const [error, setError] = useState<unknown>(null);
  const connected = state === "connected";

  const ensureTerminal = useCallback(() => {
    if (terminalRef.current) return terminalRef.current;
    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
      fontSize: 13,
      lineHeight: 1.35,
      scrollback: 3000,
      theme: { background: "#0d1117", foreground: "#f0f6fc", cursor: "#f0f6fc", selectionBackground: "#264f78" },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    if (mountRef.current) terminal.open(mountRef.current);
    fit.fit();
    terminalRef.current = terminal;
    fitRef.current = fit;
    inputDisposableRef.current = terminal.onData((value) => socketRef.current?.sendInput(value));
    resizeDisposableRef.current = terminal.onResize(({ cols, rows }) => socketRef.current?.resize(cols, rows));
    const fitToContainer = () => fit.fit();
    fitListenerRef.current = fitToContainer;
    window.addEventListener("resize", fitToContainer);
    return terminal;
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (fitListenerRef.current) window.removeEventListener("resize", fitListenerRef.current);
      inputDisposableRef.current?.dispose();
      resizeDisposableRef.current?.dispose();
      socketRef.current?.disconnect();
      socketRef.current = null;
      terminalRef.current?.dispose();
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, []);

  const connect = useCallback(async () => {
    if (!capability.available || state === "connecting" || state === "connected") return;
    setError(null);
    setState("connecting");
    socketRef.current?.disconnect();
    let sessionId: string | null = null;
    try {
      const terminal = ensureTerminal();
      const session = await terminalApi.createSession(nodeId, csrfToken);
      sessionId = session.id;
      if (!mountedRef.current) {
        await terminalApi.closeSession(session.id, csrfToken).catch(() => undefined);
        return;
      }
      fitRef.current?.fit();
      socketRef.current = openTerminalSocket(session.id, csrfToken, terminal.cols, terminal.rows, {
        onOpened: () => { setState("connected"); terminal.focus(); },
        onOutput: (bytes) => terminal.write(bytes),
        onExited: (reason, exitCode) => {
          setState("closed");
          terminal.writeln(`\r\n[会话已结束: ${exitReasonLabel(reason)}${exitCode == null ? "" : `, exit ${exitCode}`}]`);
        },
        onError: (message) => { setError(new Error(message)); setState("error"); },
      });
    } catch (nextError) {
      if (sessionId) await terminalApi.closeSession(sessionId, csrfToken).catch(() => undefined);
      setError(nextError);
      setState("error");
    }
  }, [capability.available, csrfToken, ensureTerminal, nodeId, state]);

  const close = () => {
    if (!socketRef.current) return;
    setState("closing");
    socketRef.current.close();
  };

  return <section className="node-terminal" aria-label={`${nodeName} SSH 终端`}>
    <div className="terminal-toolbar">
      <div>
        <span className={`connection-state connection-state--${state}`}>{stateLabels[state]}</span>
        <code>{nodeName}</code>
      </div>
      <div>
        <Button disabled={state === "connecting" || connected || state === "closing"} onClick={() => void connect()}>
          <Link2 aria-hidden="true" />连接终端
        </Button>
        <Button aria-label="发送 Ctrl+C" title="发送 Ctrl+C" disabled={!connected} onClick={() => socketRef.current?.sendInput("\u0003")}>
          <Keyboard aria-hidden="true" /><span>Ctrl+C</span>
        </Button>
        <Button aria-label="清空终端" title="清空终端" onClick={() => terminalRef.current?.clear()}>
          <Eraser aria-hidden="true" /><span>清屏</span>
        </Button>
        <Button aria-label="关闭终端" title="关闭终端" disabled={!connected} onClick={close}>
          <X aria-hidden="true" /><span>关闭</span>
        </Button>
      </div>
    </div>
    {error ? <div className="terminal-error"><Link2Off aria-hidden="true" /><ApiErrorNotice error={toNotice(error)} /></div> : null}
    <div ref={mountRef} className="terminal-viewport" aria-label="终端输出" />
  </section>;
}

function exitReasonLabel(reason: string) {
  const labels: Record<string, string> = {
    administrator_closed: "管理员关闭",
    administrator_request: "管理员关闭",
    process_exited: "Shell 退出",
    idle_timeout: "空闲超时",
    lifetime_exceeded: "达到最长会话时间",
    agent_disconnected: "Agent 断开",
    peer_disconnected: "链路断开",
    browser_disconnected: "浏览器断开",
    output_limit_exceeded: "输出超过限制",
    protocol_error: "终端协议错误",
    executor_unavailable: "Executor 不可用",
  };
  return labels[reason] ?? reason;
}
