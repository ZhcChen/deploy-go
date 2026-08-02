import { createParser, type EventSourceMessage } from "eventsource-parser";
import { apiFetch } from "./http-client";

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
}

export async function streamSse({ path, after = 0, signal, maxRetries = 5, onEvent, onState, fetcher = apiFetch, wait = abortableWait }: StreamSseOptions) {
  let cursor = after;
  let attempts = 0;
  while (!signal.aborted) {
    onState?.(attempts === 0 ? "connecting" : "reconnecting");
    try {
      const response = await fetcher(path, {
        headers: { Accept: "text/event-stream", ...(cursor > 0 ? { "Last-Event-ID": String(cursor) } : {}) },
        signal,
      });
      if (!response.body) throw new Error("日志响应没有可读取内容");
      onState?.("open");
      let terminal = false;
      const parser = createParser({
        onEvent(message: EventSourceMessage) {
          if (message.id) {
            const next = Number(message.id);
            if (Number.isSafeInteger(next) && next >= 0) cursor = Math.max(cursor, next);
          }
          const event = { id: message.id ?? "", event: message.event ?? "message", data: message.data };
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
