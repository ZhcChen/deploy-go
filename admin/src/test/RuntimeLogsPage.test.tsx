import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RuntimeLogsPage } from "../features/runtime-logs/RuntimeLogsPage";

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
});
