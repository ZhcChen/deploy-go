import { describe, expect, it, vi } from "vitest";
import { streamSse } from "../api/sse-client";
import { appendDeploymentLog, sanitizeLogText } from "../features/deployments/log-store";

describe("SSE 日志状态机", () => {
  it("断线后使用最后 event ID 续传且终态停止重连", async () => {
    const requests: RequestInit[] = [];
    const fetcher = vi.fn(async (_path: string, init?: RequestInit) => {
      requests.push(init ?? {});
      const body = requests.length === 1
        ? "id: 120\nevent: log\ndata: {\"sequence\":120}\n\n"
        : "event: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":120}\n\n";
      return new Response(body, { headers: { "Content-Type": "text/event-stream" } });
    });
    const events: string[] = [];
    await streamSse({
      path: "/api/v1/deployments/deployment-1/logs",
      signal: new AbortController().signal,
      fetcher,
      wait: async () => undefined,
      onEvent: (event) => events.push(event.event),
    });
    expect(events).toEqual(["log", "terminal"]);
    expect(new Headers(requests[1]?.headers).get("Last-Event-ID")).toBe("120");
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it("日志按 sequence 去重、限制窗口并替换控制字符", () => {
    const first = { sequence: 2, stream: "stdout", content: "safe<script>\u0000\u202e", truncated: false, createdAt: "2026-08-02T00:00:00Z" };
    let logs = appendDeploymentLog([], first, 2);
    logs = appendDeploymentLog(logs, first, 2);
    logs = appendDeploymentLog(logs, { ...first, sequence: 1, content: "older" }, 2);
    logs = appendDeploymentLog(logs, { ...first, sequence: 3, content: "newer" }, 2);
    expect(logs.map((log) => log.sequence)).toEqual([2, 3]);
    expect(logs[0]?.content).toBe("safe<script>��");
    expect(sanitizeLogText("line\nnext\tvalue")).toBe("line\nnext\tvalue");
  });

  it("达到重连上限后停止请求并返回错误", async () => {
    const fetcher = vi.fn(async () => new Response("", { headers: { "Content-Type": "text/event-stream" } }));
    await expect(streamSse({
      path: "/api/v1/deployments/deployment-1/logs",
      signal: new AbortController().signal,
      maxRetries: 2,
      fetcher,
      wait: async () => undefined,
      onEvent: () => undefined,
    })).rejects.toThrow("日志连接意外结束");
    expect(fetcher).toHaveBeenCalledTimes(3);
  });
});
