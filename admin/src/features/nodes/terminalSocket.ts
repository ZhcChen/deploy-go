const TERMINAL_PROTOCOL = "deploy-go-terminal.v1";
const MAX_INPUT_CHUNK_BYTES = 60 * 1024;

type ServerMessage =
  | { type: "opened"; session_id: string; sequence: number }
  | { type: "output"; session_id: string; sequence: number; encoding: "base64"; data: string }
  | { type: "exited"; session_id: string; sequence: number; reason: string; exit_code: number | null }
  | { type: "error"; code: string; message: string };

type TerminalSocketCallbacks = {
  onOpened: () => void;
  onOutput: (bytes: Uint8Array) => void;
  onExited: (reason: string, exitCode: number | null) => void;
  onError: (message: string) => void;
};

export type TerminalSocket = {
  sendInput(value: string): void;
  resize(columns: number, rows: number): void;
  close(): void;
  disconnect(): void;
};

export function openTerminalSocket(
  sessionId: string,
  csrfToken: string,
  columns: number,
  rows: number,
  callbacks: TerminalSocketCallbacks,
): TerminalSocket {
  const socket = new WebSocket(terminalWebSocketUrl(sessionId), [TERMINAL_PROTOCOL, `csrf.${csrfToken}`]);
  let sequence = 0;
  let terminal = false;
  let opened = false;
  let expectedServerSequence = 1;
  let expectedClose = false;
  let closeSent = false;
  const send = (message: object) => {
    if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message));
  };
  const fail = (message: string) => {
    if (terminal) return;
    terminal = true;
    callbacks.onError(message);
    socket.close();
  };

  socket.onopen = () => send({ type: "open", columns, rows });
  socket.onmessage = (event) => {
    let message: ServerMessage;
    try {
      message = JSON.parse(String(event.data)) as ServerMessage;
    } catch {
      fail("终端返回了无法识别的消息");
      return;
    }
    if (message.type !== "error") {
      if (message.session_id !== sessionId || message.sequence !== expectedServerSequence) {
        fail("终端返回了无效的会话序号");
        return;
      }
      expectedServerSequence += 1;
    }
    if (message.type === "opened") {
      if (opened) { fail("终端重复确认连接"); return; }
      opened = true;
      callbacks.onOpened();
    }
    else if (message.type === "output" && message.encoding === "base64") {
      if (!opened) { fail("终端在连接确认前返回了输出"); return; }
      try { callbacks.onOutput(decodeBase64(message.data)); }
      catch { fail("终端输出解码失败"); }
    } else if (message.type === "exited") {
      terminal = true;
      callbacks.onExited(message.reason, message.exit_code);
    } else if (message.type === "error") fail(message.message);
  };
  socket.onerror = () => {
    if (!expectedClose && !terminal) fail("终端连接异常中断");
  };
  socket.onclose = () => {
    if (terminal) return;
    if (expectedClose) {
      terminal = true;
      callbacks.onExited("administrator_closed", null);
    } else fail("终端连接异常中断");
  };

  return {
    sendInput(value) {
      if (!value) return;
      for (const data of encodeBase64Chunks(value)) {
        send({ type: "input", sequence: ++sequence, encoding: "base64", data });
      }
    },
    resize(nextColumns, nextRows) {
      send({ type: "resize", sequence: ++sequence, columns: nextColumns, rows: nextRows });
    },
    close() {
      if (socket.readyState !== WebSocket.OPEN || closeSent) return;
      expectedClose = true;
      closeSent = true;
      send({ type: "close", sequence: ++sequence });
    },
    disconnect() {
      expectedClose = true;
      if (socket.readyState === WebSocket.OPEN && !closeSent && !terminal) {
        closeSent = true;
        send({ type: "close", sequence: ++sequence });
      }
      socket.close();
    },
  };
}

function terminalWebSocketUrl(sessionId: string) {
  const configuredBase = import.meta.env.VITE_API_BASE_URL?.replace(/\/+$/, "");
  const path = `/api/v1/terminal-sessions/${encodeURIComponent(sessionId)}/stream`;
  const url = new URL(`${configuredBase ?? ""}${path}`, window.location.origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
}

function encodeBase64Chunks(value: string) {
  const bytes = new TextEncoder().encode(value);
  const chunks: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += MAX_INPUT_CHUNK_BYTES) {
    chunks.push(encodeBase64(bytes.subarray(offset, offset + MAX_INPUT_CHUNK_BYTES)));
  }
  return chunks;
}

function encodeBase64(bytes: Uint8Array) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function decodeBase64(value: string) {
  const binary = atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}
