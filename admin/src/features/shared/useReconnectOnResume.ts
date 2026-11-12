import { useEffect, useRef } from "react";

/**
 * `enabled` 期间，浏览器恢复联网或标签页回到前台时调用一次 `reconnect`。
 *
 * 用于已经放弃自动重连、停在“连接已断开”的实时流：笔记本休眠恢复、网络切换、切回标签页，
 * 正是最该重试的时机，不应该让用户去找“重新连接”按钮。`reconnect` 之后 `enabled` 应当变回假，
 * 所以一次事件只会触发一次重连；真正的重连仍带指数退避，不会形成热循环。
 */
export function useReconnectOnResume(enabled: boolean, reconnect: () => void) {
  const latest = useRef(reconnect);
  useEffect(() => {
    latest.current = reconnect;
  }, [reconnect]);

  useEffect(() => {
    if (!enabled) return;
    const onOnline = () => latest.current();
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") latest.current();
    };
    window.addEventListener("online", onOnline);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      window.removeEventListener("online", onOnline);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [enabled]);
}
