import type { QueryClient } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  MAX_CONSECUTIVE_FAILURES,
  RECONNECT_BASE_MS,
  RECONNECT_MAX_MS,
  reconnectDelay,
  useNodeStatusSocket,
} from "../features/nodes/useNodeStatusSocket";

class SocketDouble {
  static instances: SocketDouble[] = [];
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;

  constructor(readonly url: string, readonly protocols: string[]) {
    SocketDouble.instances.push(this);
  }

  /** 与浏览器一致：close 只触发一次 close 事件。 */
  close() {
    if (this.closed) return;
    this.closed = true;
    this.onclose?.();
  }

  /** 服务端消息到达。 */
  receive(message: unknown) {
    this.onmessage?.({ data: typeof message === "string" ? message : JSON.stringify(message) });
  }

  /** 握手被拒或网络不通：浏览器先触发 error，再触发 close，且从未收到过消息。 */
  reject() {
    this.onerror?.();
  }
}

function setVisibility(state: "visible" | "hidden") {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
}

const invalidateQueries = vi.fn();
const queryClient = { invalidateQueries } as unknown as QueryClient;
const attempts = () => SocketDouble.instances.length;
const latest = () => SocketDouble.instances.at(-1)!;

/** 前 9 次失败之后各自等待的间隔：第 10 次失败后不再安排重试。 */
const backoffSchedule = Array.from({ length: MAX_CONSECUTIVE_FAILURES - 1 }, (_, index) => reconnectDelay(index + 1));

describe("节点状态 WebSocket 的重连", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    SocketDouble.instances = [];
    invalidateQueries.mockClear();
    vi.stubGlobal("WebSocket", SocketDouble);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    setVisibility("visible");
  });

  it("退避间隔从 3 秒起倍增，上限 60 秒", () => {
    expect(backoffSchedule).toEqual([3_000, 6_000, 12_000, 24_000, 48_000, 60_000, 60_000, 60_000, 60_000]);
    expect(reconnectDelay(0)).toBe(RECONNECT_BASE_MS);
    expect(reconnectDelay(99)).toBe(RECONNECT_MAX_MS);
  });

  it("握手被拒时按连续失败指数退避，达到上限后停止重试", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    expect(attempts()).toBe(1);
    expect(latest().protocols).toEqual(["admin-node-status.v1", "csrf.csrf-token"]);

    for (const [index, delay] of backoffSchedule.entries()) {
      latest().reject();
      vi.advanceTimersByTime(delay - 1);
      expect(attempts(), `第 ${index + 1} 次失败后在退避结束前不应重试`).toBe(index + 1);
      vi.advanceTimersByTime(1);
      expect(attempts(), `第 ${index + 1} 次失败后退避结束应重试`).toBe(index + 2);
    }

    // 第 10 次失败后不再安排重试，无论过去多久。
    latest().reject();
    vi.advanceTimersByTime(60 * 60_000);
    expect(attempts()).toBe(MAX_CONSECUTIVE_FAILURES);
  });

  it("连接成功（收到服务端消息）后断开，从最短间隔重新开始", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    SocketDouble.instances[0].reject();
    vi.advanceTimersByTime(3_000);
    SocketDouble.instances[1].reject();
    vi.advanceTimersByTime(6_000);
    expect(attempts()).toBe(3);

    // 第三个连接握手成功，之后断开：累计的失败作废，3 秒后重试。
    SocketDouble.instances[2].receive({ type: "hello" });
    SocketDouble.instances[2].close();
    vi.advanceTimersByTime(RECONNECT_BASE_MS - 1);
    expect(attempts()).toBe(3);
    vi.advanceTimersByTime(1);
    expect(attempts()).toBe(4);

    // 计数确实清零了：这次再失败只等 6 秒，而不是接着此前的 3 次累计成 24 秒。
    SocketDouble.instances[3].reject();
    vi.advanceTimersByTime(reconnectDelay(2) - 1);
    expect(attempts()).toBe(4);
    vi.advanceTimersByTime(1);
    expect(attempts()).toBe(5);
  });

  it("联网时立即重试，并重置失败计数", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    SocketDouble.instances[0].reject();
    vi.advanceTimersByTime(3_000);
    SocketDouble.instances[1].reject();
    vi.advanceTimersByTime(6_000);
    SocketDouble.instances[2].reject(); // 此刻在等 12 秒
    expect(attempts()).toBe(3);

    window.dispatchEvent(new Event("online"));
    expect(attempts()).toBe(4);

    // 计数已重置：新连接失败后只等 3 秒。
    latest().reject();
    vi.advanceTimersByTime(RECONNECT_BASE_MS - 1);
    expect(attempts()).toBe(4);
    vi.advanceTimersByTime(1);
    expect(attempts()).toBe(5);
  });

  it("标签页回到前台时立即重试，切到后台不触发", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    latest().reject();
    expect(attempts()).toBe(1);

    setVisibility("hidden");
    document.dispatchEvent(new Event("visibilitychange"));
    expect(attempts()).toBe(1);

    setVisibility("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    expect(attempts()).toBe(2);
  });

  it("已经放弃重试之后，回到前台会再试一次", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    for (const delay of backoffSchedule) {
      latest().reject();
      vi.advanceTimersByTime(delay);
    }
    latest().reject();
    vi.advanceTimersByTime(60 * 60_000);
    expect(attempts()).toBe(MAX_CONSECUTIVE_FAILURES);

    document.dispatchEvent(new Event("visibilitychange"));

    expect(attempts()).toBe(MAX_CONSECUTIVE_FAILURES + 1);
  });

  it("已有连接（含握手中和已连接）时，联网和回到前台不重复建立连接", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    window.dispatchEvent(new Event("online"));
    document.dispatchEvent(new Event("visibilitychange"));
    expect(attempts()).toBe(1);

    latest().receive({ type: "hello" });
    window.dispatchEvent(new Event("online"));
    document.dispatchEvent(new Event("visibilitychange"));
    expect(attempts()).toBe(1);
  });

  it("卸载后清理：不再重试，不再响应联网，摘掉监听，已建立的连接被关闭", () => {
    const removeWindowListener = vi.spyOn(window, "removeEventListener");
    const removeDocumentListener = vi.spyOn(document, "removeEventListener");
    const waiting = renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    latest().reject();
    expect(vi.getTimerCount()).toBe(1);
    waiting.unmount();
    // 行为上被 disposed 守卫掩盖（遗留的定时器触发后什么也不做），所以直接断言定时器和监听确实被清掉。
    expect(vi.getTimerCount()).toBe(0);
    expect(removeWindowListener.mock.calls.map(([type]) => type)).toContain("online");
    expect(removeDocumentListener.mock.calls.map(([type]) => type)).toContain("visibilitychange");
    vi.advanceTimersByTime(60 * 60_000);
    window.dispatchEvent(new Event("online"));
    expect(attempts()).toBe(1);

    SocketDouble.instances = [];
    const connected = renderHook(() => useNodeStatusSocket(queryClient, "csrf-token"));
    latest().receive({ type: "hello" });
    connected.unmount();
    expect(latest().closed).toBe(true);
    vi.advanceTimersByTime(60 * 60_000);
    expect(attempts()).toBe(1);
  });

  it("没有令牌（未登录或不是管理员）时不建立连接", () => {
    renderHook(() => useNodeStatusSocket(queryClient, null));
    renderHook(() => useNodeStatusSocket(queryClient, undefined));
    renderHook(() => useNodeStatusSocket(queryClient, ""));
    window.dispatchEvent(new Event("online"));
    expect(attempts()).toBe(0);
  });

  it("收到失效通知时让节点与 Agent 查询重新拉取，无法识别的消息被忽略", () => {
    renderHook(() => useNodeStatusSocket(queryClient, "csrf-token", "node-1"));
    latest().receive({ type: "hello" });
    expect(invalidateQueries.mock.calls.map(([options]) => options.queryKey)).toEqual([
      ["nodes"], ["agents"], ["node", "node-1"], ["agents", "node", "node-1"],
    ]);

    invalidateQueries.mockClear();
    latest().receive({ type: "something_else" });
    latest().receive("不是 JSON");
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(attempts()).toBe(1);
  });
});
