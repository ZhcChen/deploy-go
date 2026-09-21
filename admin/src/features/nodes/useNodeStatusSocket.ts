import { useEffect } from "react";
import type { QueryClient } from "@tanstack/react-query";

const PROTOCOL = "admin-node-status.v1";

function socketUrl() {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/api/v1/admin/node-status`;
}

export function useNodeStatusSocket(
  queryClient: QueryClient,
  csrfToken: string | null | undefined,
  nodeId?: string,
) {
  useEffect(() => {
    if (!csrfToken || typeof WebSocket === "undefined") return;
    let socket: WebSocket | null = null;
    let reconnectTimer: number | undefined;
    let disposed = false;

    const connect = () => {
      if (disposed) return;
      socket = new WebSocket(socketUrl(), [PROTOCOL, `csrf.${csrfToken}`]);
      socket.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as { type?: string };
          if (message.type === "snapshot_invalidated" || message.type === "hello") {
            void queryClient.invalidateQueries({ queryKey: ["nodes"] });
            void queryClient.invalidateQueries({ queryKey: ["agents"] });
            if (nodeId) {
              void queryClient.invalidateQueries({ queryKey: ["node", nodeId] });
              void queryClient.invalidateQueries({ queryKey: ["agents", "node", nodeId] });
            }
          }
        } catch {
          // WS 只负责失效通知，HTTP 轮询仍是权威状态来源。
        }
      };
      socket.onclose = () => {
        socket = null;
        if (!disposed) reconnectTimer = window.setTimeout(connect, 3000);
      };
      socket.onerror = () => socket?.close();
    };

    connect();
    return () => {
      disposed = true;
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
      socket?.close();
    };
  }, [csrfToken, nodeId, queryClient]);
}
