import { Eraser, Pause, Play, RotateCw, Search } from "lucide-react";
import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import { streamSse, type SseConnectionState } from "../../api/sse-client";
import { Button } from "../../components/Button";
import { Field, Select, TextInput } from "../../components/form";

interface RuntimeLog { sequence: number; timestamp: string; level: string; target: string; message: string; request_id?: string; fields: Record<string, unknown>; }
interface Filters { level: string; requestId: string; target: string; }
const emptyFilters: Filters = { level: "", requestId: "", target: "" };
const stateLabels: Record<SseConnectionState | "disconnected", string> = { connecting: "连接中", open: "实时", reconnecting: "重连中", ended: "已结束", disconnected: "已断开" };

function logPath(filters: Filters) {
  const query = new URLSearchParams();
  if (filters.level) query.set("level", filters.level);
  if (filters.requestId) query.set("request_id", filters.requestId);
  if (filters.target) query.set("target", filters.target);
  return `/api/v1/runtime-logs${query.size ? `?${query}` : ""}`;
}

export function RuntimeLogsPage() {
  const [draft, setDraft] = useState<Filters>(emptyFilters);
  const [filters, setFilters] = useState<Filters>(emptyFilters);
  const [logs, setLogs] = useState<RuntimeLog[]>([]);
  const [connection, setConnection] = useState<SseConnectionState | "disconnected">("connecting");
  const [following, setFollowing] = useState(true);
  const [dropped, setDropped] = useState(0);
  const [generation, setGeneration] = useState(0);
  const viewport = useRef<HTMLDivElement>(null);
  const lastSequence = useRef(0);
  const path = useMemo(() => logPath(filters), [filters]);

  useEffect(() => {
    const controller = new AbortController();
    void streamSse({ path, after: lastSequence.current, signal: controller.signal, maxRetries: 8, onState: setConnection, onEvent(event) {
      if (event.event === "log") {
        try {
          const log = JSON.parse(event.data) as RuntimeLog;
          if (!Number.isSafeInteger(log.sequence)) return;
          lastSequence.current = Math.max(lastSequence.current, log.sequence);
          setLogs((current) => [...current, log].slice(-2_000));
        } catch { /* 无效事件不应中断日志流。 */ }
      } else if (event.event === "stats") {
        try { setDropped(Number((JSON.parse(event.data) as { dropped?: number }).dropped ?? 0)); } catch { /* ignore */ }
      }
    } }).catch(() => { if (!controller.signal.aborted) setConnection("disconnected"); });
    return () => controller.abort();
  }, [generation, path]);

  useEffect(() => { if (following && viewport.current) viewport.current.scrollTop = viewport.current.scrollHeight; }, [following, logs]);

  function applyFilters(event: FormEvent) {
    event.preventDefault();
    lastSequence.current = 0;
    setLogs([]);
    setConnection("connecting");
    setFilters({ level: draft.level, requestId: draft.requestId.trim(), target: draft.target.trim() });
  }

  return <section className="workspace runtime-logs-page">
    <div className="workspace-heading"><div><h2>运行日志</h2><p>实时查看 API stdout 对应的结构化事件。日志保存在当前进程内存中，服务重启后清空。</p></div><span className={`connection-state connection-state--${connection}`}>{stateLabels[connection]}</span></div>
    <form className="filter-bar runtime-log-filters" onSubmit={applyFilters}>
      <Field label="级别"><Select value={draft.level} onChange={(event) => setDraft((value) => ({ ...value, level: event.target.value }))}><option value="">全部</option><option value="INFO">INFO</option><option value="WARN">WARN</option><option value="ERROR">ERROR</option><option value="DEBUG">DEBUG</option><option value="TRACE">TRACE</option></Select></Field>
      <Field label="Request ID"><TextInput value={draft.requestId} onChange={(event) => setDraft((value) => ({ ...value, requestId: event.target.value }))} placeholder="req_01..." /></Field>
      <Field label="Target"><TextInput value={draft.target} onChange={(event) => setDraft((value) => ({ ...value, target: event.target.value }))} placeholder="deploy_go_api::auth" /></Field>
      <Button type="submit"><Search aria-hidden="true" />筛选</Button>
    </form>
    <div className="log-workspace runtime-log-workspace">
      <div className="log-toolbar"><div><strong>API 事件流</strong><span>{logs.length} 条{dropped > 0 ? ` · 采集队列已丢弃 ${dropped} 条` : ""}</span></div><div>
        <Button title={following ? "暂停自动跟随" : "恢复自动跟随"} onClick={() => setFollowing((value) => !value)}>{following ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}{following ? "暂停" : "跟随"}</Button>
        {connection === "disconnected" ? <Button onClick={() => { setConnection("connecting"); setGeneration((value) => value + 1); }}><RotateCw aria-hidden="true" />重连</Button> : null}
        <Button onClick={() => setLogs([])}><Eraser aria-hidden="true" />清空视图</Button>
      </div></div>
      <div className="log-viewport runtime-log-viewport" ref={viewport} data-testid="runtime-log"><pre>{logs.length === 0 ? <span className="log-empty">等待运行日志...</span> : logs.map((log) => <span className={`runtime-log-line runtime-log-line--${log.level.toLowerCase()}`} key={log.sequence}><i>{log.sequence}</i><time>{new Date(log.timestamp).toLocaleTimeString()}</time><b>{log.level}</b><code>{log.target}</code><span>{log.message}{log.request_id ? ` · ${log.request_id}` : ""}{Object.keys(log.fields).length ? ` · ${JSON.stringify(log.fields)}` : ""}</span>{"\n"}</span>)}</pre></div>
    </div>
  </section>;
}
