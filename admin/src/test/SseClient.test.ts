import { describe, expect, it, vi } from "vitest";
import { streamSse } from "../api/sse-client";
import { appendDeploymentLog, classifyDeploymentLog, formatDeploymentLogs, sanitizeLogText } from "../features/deployments/log-store";

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

  it("复制日志时保留阶段、序号和输出流", () => {
    const base = { stream: "stdout", truncated: false, createdAt: "2026-08-02T00:00:00Z" };
    expect(formatDeploymentLogs([
      { ...base, sequence: 1, stage: "prepare", content: "prepare output" },
      { ...base, sequence: 2, stage: "release", stream: "stderr", content: "release error", truncated: true },
    ])).toMatch(/^准备阶段（prepare）\n\d{2}\/\d{2} \d{2}:\d{2}:\d{2}\t1\tstdout\tprepare output\n\n发布阶段（release）\n\d{2}\/\d{2} \d{2}:\d{2}:\d{2}\t2\tstderr\trelease error \[已截断\]$/);
  });

  it("区分普通输出、结构化进度、诊断输出和失败事件", () => {
    expect(classifyDeploymentLog({ stream: "stdout", content: "build complete" })).toBe("output");
    expect(classifyDeploymentLog({ stream: "stderr", content: "Container api Starting" })).toBe("diagnostic");
    expect(classifyDeploymentLog({ stream: "stdout", content: 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.succeeded"}' })).toBe("progress");
    expect(classifyDeploymentLog({ stream: "stdout", content: 'before\nDEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.step.failed"}\nafter' })).toBe("error");
    expect(classifyDeploymentLog({ stream: "stderr", content: 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.module.failed"}' })).toBe("error");
    expect(classifyDeploymentLog({ stream: "stderr", content: "DEPLOY_GO_EVENT not-json" })).toBe("diagnostic");
    expect(classifyDeploymentLog({ stream: "stdout", content: 'DEPLOY_GO_EVENT {"schema_version":1,"event":"deploy.unknown"}' })).toBe("output");
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

describe("SSE 重连计数按连续失败计", () => {
  const stream = (body: string) => new Response(body, { headers: { "Content-Type": "text/event-stream" } });
  const base = { path: "/api/v1/deployments/deployment-1/logs", signal: new AbortController().signal, onEvent: () => undefined };

  it("每次连接都收到过日志时计数重新开始：断线再多次也不会累计放弃", async () => {
    const delays: number[] = [];
    const events: string[] = [];
    let calls = 0;
    const fetcher = vi.fn(async () => {
      calls += 1;
      // 前 8 次连接各收到一条日志后意外断开，第 9 次收到终态。
      return stream(calls <= 8
        ? `id: ${calls}\nevent: log\ndata: {"sequence":${calls}}\n\n`
        : 'event: terminal\ndata: {"status":"succeeded","last_event_id":8}\n\n');
    });
    await streamSse({
      ...base,
      maxRetries: 2,
      fetcher,
      wait: async (milliseconds) => { delays.push(milliseconds); },
      onEvent: (event) => events.push(event.event),
    });
    expect(fetcher).toHaveBeenCalledTimes(9);
    expect(events.at(-1)).toBe("terminal");
    // 每次断线后的退避都从最短开始，而不是越拖越长。
    expect(delays).toEqual(Array(8).fill(1000));
  });

  it("连上后只收到 stream-error 不算健康，仍按连续失败上限停止", async () => {
    let calls = 0;
    const fetcher = vi.fn(async () => {
      calls += 1;
      // 保险：实现一旦把这种连接当成健康而无限重连，明确失败，而不是让测试挂住。
      if (calls > 50) throw new Error("重连失控");
      return stream("event: stream-error\ndata: 日志读取暂时失败\n\n");
    });
    await expect(streamSse({ ...base, maxRetries: 3, fetcher, wait: async () => undefined })).rejects.toThrow("日志连接意外结束");
    expect(fetcher).toHaveBeenCalledTimes(4);
  });

  it("没有日志但连接保持不少于 10 秒也视为健康（只有保活和统计的流）", async () => {
    let clock = 0;
    // 每次读取时间前进 12 秒：连接建立到断开之间经过了 12 秒。
    const now = () => (clock += 12_000);
    let calls = 0;
    const fetcher = vi.fn(async () => {
      calls += 1;
      return stream(calls <= 6 ? ": keep-alive\n\n" : 'event: terminal\ndata: {"status":"succeeded","last_event_id":0}\n\n');
    });
    await streamSse({ ...base, maxRetries: 2, fetcher, wait: async () => undefined, now });
    expect(fetcher).toHaveBeenCalledTimes(7);
  });

  it("连接保持不到 10 秒就断开仍计入连续失败", async () => {
    let clock = 0;
    const now = () => (clock += 1_000);
    const fetcher = vi.fn(async () => stream(": keep-alive\n\n"));
    await expect(streamSse({ ...base, maxRetries: 2, fetcher, wait: async () => undefined, now })).rejects.toThrow("日志连接意外结束");
    expect(fetcher).toHaveBeenCalledTimes(3);
  });
});
