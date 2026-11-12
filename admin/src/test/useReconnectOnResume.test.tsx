import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useReconnectOnResume } from "../features/shared/useReconnectOnResume";

function setVisibility(state: "visible" | "hidden") {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
}

describe("断开后联网或回到前台自动重连", () => {
  afterEach(() => setVisibility("visible"));

  it("启用时，联网和标签页回到前台各触发一次", () => {
    const reconnect = vi.fn();
    renderHook(() => useReconnectOnResume(true, reconnect));

    window.dispatchEvent(new Event("online"));
    expect(reconnect).toHaveBeenCalledTimes(1);

    setVisibility("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    expect(reconnect).toHaveBeenCalledTimes(2);
  });

  it("标签页切到后台时不触发", () => {
    const reconnect = vi.fn();
    renderHook(() => useReconnectOnResume(true, reconnect));

    setVisibility("hidden");
    document.dispatchEvent(new Event("visibilitychange"));

    expect(reconnect).not.toHaveBeenCalled();
  });

  it("未启用时不监听，启用后才监听，再关闭后停止监听", () => {
    const reconnect = vi.fn();
    const { rerender } = renderHook(({ enabled }) => useReconnectOnResume(enabled, reconnect), { initialProps: { enabled: false } });
    window.dispatchEvent(new Event("online"));
    expect(reconnect).not.toHaveBeenCalled();

    rerender({ enabled: true });
    window.dispatchEvent(new Event("online"));
    expect(reconnect).toHaveBeenCalledTimes(1);

    // 重连发起后状态不再是“已断开”，监听随之关闭，不会重复触发。
    rerender({ enabled: false });
    window.dispatchEvent(new Event("online"));
    expect(reconnect).toHaveBeenCalledTimes(1);
  });

  it("始终调用最新的回调，卸载后清理监听", () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender, unmount } = renderHook(({ callback }) => useReconnectOnResume(true, callback), { initialProps: { callback: first } });
    rerender({ callback: second });

    window.dispatchEvent(new Event("online"));
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);

    unmount();
    window.dispatchEvent(new Event("online"));
    expect(second).toHaveBeenCalledTimes(1);
  });
});
