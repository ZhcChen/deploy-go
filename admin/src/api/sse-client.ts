import { createParser, type EventSourceMessage } from "eventsource-parser";
import { ApiError, apiFetch } from "./http-client";

export type SseConnectionState = "connecting" | "open" | "reconnecting" | "ended";

export interface SseEvent {
  id: string;
  event: string;
  data: string;
}

interface StreamSseOptions {
  path: string;
  after?: number;
  signal: AbortSignal;
  maxRetries?: number;
  onEvent(event: SseEvent): void;
  onState?(state: SseConnectionState): void;
  fetcher?: typeof apiFetch;
  wait?(milliseconds: number, signal: AbortSignal): Promise<void>;
  now?(): number;
}

/** 连接保持这么久就视为健康：服务端每 15 秒发一次保活，能撑过 10 秒说明连接是通的。 */
const HEALTHY_CONNECTION_MS = 10_000;

/**
 * 读取 SSE 流并在断线后按最后的 event ID 续传。
 *
 * `maxRetries` 限制的是“连续失败”次数，不是整个页面生命周期内的断线总数：一次部署可能持续很久，
 * 期间 API 重启、网络抖动都会断线，只要中间连上并拿到了数据（收到过日志事件，或连接保持了足够久），
 * 失败计数就从零开始。连上就断、或者只收到 `stream-error` 的情况仍然计入，超过上限后停止，不会热循环。
 */
export async function streamSse({ path, after = 0, signal, maxRetries = 5, onEvent, onState, fetcher = apiFetch, wait = abortableWait, now = Date.now }: StreamSseOptions) {
  let cursor = after;
  let attempts = 0;
  while (!signal.aborted) {
    onState?.(attempts === 0 ? "connecting" : "reconnecting");
    let openedAt: number | undefined;
    let receivedLog = false;
    try {
      const response = await fetcher(path, {
        headers: { Accept: "text/event-stream", ...(cursor > 0 ? { "Last-Event-ID": String(cursor) } : {}) },
        signal,
      });
      if (!response.body) throw new Error("日志响应没有可读取内容");
      onState?.("open");
      openedAt = now();
      let terminal = false;
      const parser = createParser({
        onEvent(message: EventSourceMessage) {
          if (message.id) {
            const next = Number(message.id);
            if (Number.isSafeInteger(next) && next >= 0) cursor = Math.max(cursor, next);
          }
          const event = { id: message.id ?? "", event: message.event ?? "message", data: message.data };
          if (event.event === "log") receivedLog = true;
          onEvent(event);
          if (event.event === "terminal" || event.event === "authorization-revoked") terminal = true;
        },
      });
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      while (!signal.aborted && !terminal) {
        const { done, value } = await reader.read();
        if (done) break;
        parser.feed(decoder.decode(value, { stream: true }));
      }
      parser.feed(decoder.decode());
      await reader.cancel().catch(() => undefined);
      if (terminal || signal.aborted) {
        onState?.("ended");
        return;
      }
      throw new Error("日志连接意外结束");
    } catch (error) {
      if (signal.aborted) return;
      if (error instanceof ApiError && (error.status === 401 || error.status === 403)) throw error;
      // 这条连接是健康的：失败计数按“连续失败”重新开始。
      if (receivedLog || (openedAt !== undefined && now() - openedAt >= HEALTHY_CONNECTION_MS)) attempts = 0;
      if (attempts >= maxRetries) throw error;
      const delay = Math.min(1000 * 2 ** attempts, 8000);
      attempts += 1;
      await wait(delay, signal);
    }
  }
}

function abortableWait(milliseconds: number, signal: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    const onAbort = () => {
      window.clearTimeout(timeout);
      reject(new DOMException("Aborted", "AbortError"));
    };
    const timeout = window.setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, milliseconds);
    signal.addEventListener("abort", onAbort, { once: true });
  });
}
