import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RuntimeLogsPage } from "../features/runtime-logs/RuntimeLogsPage";
import userEvent from "@testing-library/user-event";

const streamSse = vi.fn();
vi.mock("../api/sse-client", () => ({ streamSse: (...args: unknown[]) => streamSse(...args) }));

describe("RuntimeLogsPage", () => {
  beforeEach(() => {
    streamSse.mockReset();
    streamSse.mockImplementation(async ({ onState, onEvent }) => {
      onState("open");
      onEvent({ id: "7", event: "log", data: JSON.stringify({
        sequence: 7,
        timestamp: "2026-08-06T05:00:00.000Z",
        level: "INFO",
        target: "deploy_go_api",
        message: "request completed",
        request_id: "req_01TEST",
        fields: { status: 200, elapsed_ms: 12 },
      }) });
      return new Promise(() => undefined);
    });
  });

  it("展示结构化运行日志和请求 ID", async () => {
    render(<RuntimeLogsPage />);
    expect(await screen.findByText(/request completed · req_01TEST/)).toBeInTheDocument();
    expect(screen.getByText("deploy_go_api")).toBeInTheDocument();
    expect(screen.getByText("实时")).toBeInTheDocument();
  });

  it("按节点与组件筛选，并提示轮转缺口", async () => {
    render(<RuntimeLogsPage />);
    await screen.findByText("实时");
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("节点 ID"), "node_fixture");
    await user.click(screen.getByLabelText("组件"));
    await user.click(screen.getByRole("option", { name: "Executor" }));
    await user.click(screen.getByRole("button", { name: "筛选" }));
    const options = streamSse.mock.calls.at(-1)?.[0];
    expect(options.path).toContain("node_id=node_fixture");
    expect(options.path).toContain("component=executor");
    expect(options.after).toBe(0);
    act(() => options.onEvent({ event: "gap", data: "{}" }));
    expect(await screen.findByRole("status")).toHaveTextContent("按容量轮转覆盖");
    act(() => options.onEvent({ event: "stats", data: JSON.stringify({ dropped: 3, write_errors: 2 }) }));
    expect(screen.getByText(/采集队列已丢弃 3 条 · 持久化失败 2 次/)).toBeInTheDocument();
  });
});
