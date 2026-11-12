import { useEffect } from "react";
import type { QueryClient } from "@tanstack/react-query";

const PROTOCOL = "admin-node-status.v1";

/** 重连按连续失败指数退避：3、6、12、24、48 秒，之后每次 60 秒。 */
export const RECONNECT_BASE_MS = 3_000;
export const RECONNECT_MAX_MS = 60_000;
/** 连续失败这么多次后停止重试，等浏览器恢复联网或标签页回到前台再试。 */
export const MAX_CONSECUTIVE_FAILURES = 10;

export function reconnectDelay(failures: number) {
  return Math.min(RECONNECT_BASE_MS * 2 ** Math.max(failures - 1, 0), RECONNECT_MAX_MS);
}

function socketUrl() {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/api/v1/admin/node-status`;
}

/**
 * 订阅节点与 Agent 状态的失效通知，通知到来时让相关查询重新拉取。
 *
 * 浏览器拿不到 WebSocket 握手失败的 HTTP 状态，服务端拒绝（该入口只允许管理员，并校验来源和 CSRF
 * 令牌）与网络抖动无法区分，所以重连不能固定间隔无限重试：一个令牌失效的旧标签页会每 3 秒被拒绝一次，
 * 直到关闭。这里按连续失败退避，收到服务端的第一条消息才算连接成功并清零；连续失败达到上限后停止，
 * 浏览器恢复联网或标签页回到前台时再试一次。通知只是加速刷新，页面另有轮询兜底，放弃重试不会让页面停止更新。
 */
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
    let failures = 0;

    const clearTimer = () => {
      if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer);
      reconnectTimer = undefined;
    };

    const scheduleReconnect = () => {
      clearTimer();
      if (disposed || failures >= MAX_CONSECUTIVE_FAILURES) return;
      reconnectTimer = window.setTimeout(connect, reconnectDelay(failures));
    };

    function connect() {
      reconnectTimer = undefined;
      if (disposed || socket) return;
      const current = new WebSocket(socketUrl(), [PROTOCOL, `csrf.${csrfToken}`]);
      socket = current;
      let confirmed = false;
      current.onmessage = (event) => {
        // 服务端升级成功后立即发 hello，收到消息才说明握手和鉴权都通过了。
        confirmed = true;
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
      current.onclose = () => {
        if (socket !== current) return;
        socket = null;
        // 成功连接过的连接断开，从最短间隔重新开始；没收到过消息的（握手被拒、网络不通）累计失败。
        failures = confirmed ? 1 : failures + 1;
        scheduleReconnect();
      };
      current.onerror = () => current.close();
    }

    // 浏览器恢复联网或标签页回到前台：重置失败计数并立即重试，包括已经放弃的情况。
    // 已有连接（握手中或已连接）时 connect 什么也不做，不会重复建立。
    const resume = () => {
      if (disposed) return;
      failures = 0;
      clearTimer();
      connect();
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") resume();
    };
    window.addEventListener("online", resume);
    document.addEventListener("visibilitychange", onVisibilityChange);

    connect();
    return () => {
      disposed = true;
      clearTimer();
      window.removeEventListener("online", resume);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      const current = socket;
      socket = null;
      current?.close();
    };
  }, [csrfToken, nodeId, queryClient]);
}
