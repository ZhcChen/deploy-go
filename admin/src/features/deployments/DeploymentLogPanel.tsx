import { Check, Copy } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { DeploymentLogResponseFromJSON } from "../../api/generated/models/DeploymentLogResponse";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { streamSse, type SseConnectionState } from "../../api/sse-client";
import { appendDeploymentLog, formatDeploymentLogs, sanitizeLogText } from "./log-store";

const connectionLabels: Record<SseConnectionState | "disconnected" | "revoked", string> = {
  connecting: "连接中",
  open: "实时连接",
  reconnecting: "正在重连",
  ended: "已结束",
  disconnected: "连接已断开",
  revoked: "访问授权已失效",
};

export function DeploymentLogPanel({ deploymentId, onTerminal, onAuthorizationRevoked }: { deploymentId: string; onTerminal(): void; onAuthorizationRevoked(error: ApiError): void }) {
  const [logs, setLogs] = useState<ReturnType<typeof DeploymentLogResponseFromJSON>[]>([]);
  const [connection, setConnection] = useState<SseConnectionState | "disconnected" | "revoked">("connecting");
  const [message, setMessage] = useState("");
  const [following, setFollowing] = useState(true);
  const [generation, setGeneration] = useState(0);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");
  const viewport = useRef<HTMLDivElement>(null);
  const lastSequence = useRef(0);
  const terminalCallback = useRef(onTerminal);

  useEffect(() => {
    terminalCallback.current = onTerminal;
  }, [onTerminal]);

  useEffect(() => {
    const controller = new AbortController();
    void streamSse({
      path: `/api/v1/deployments/${encodeURIComponent(deploymentId)}/logs`,
      after: lastSequence.current,
      signal: controller.signal,
      onState: setConnection,
      onEvent(event) {
        if (event.event === "log") {
          try {
            const log = DeploymentLogResponseFromJSON(JSON.parse(event.data));
            if (Number.isSafeInteger(log.sequence)) {
              lastSequence.current = Math.max(lastSequence.current, log.sequence);
              setLogs((current) => appendDeploymentLog(current, log));
            }
          } catch {
            setMessage("收到无法识别的日志事件，已忽略。请使用 Request ID 排查服务端输出。");
          }
        } else if (event.event === "stream-error") {
          setMessage("日志读取暂时失败，正在按最后游标重连。");
        } else if (event.event === "authorization-revoked") {
          let requestId: string | undefined;
          try {
            requestId = (JSON.parse(event.data) as { request_id?: string }).request_id;
          } catch {
            requestId = undefined;
          }
          setLogs([]);
          lastSequence.current = 0;
          setConnection("revoked");
          setMessage("日志访问授权已经失效，已停止接收新内容。");
          onAuthorizationRevoked(new ApiError(403, "forbidden", "日志访问授权已失效", requestId));
        } else if (event.event === "terminal") {
          setConnection("ended");
          terminalCallback.current();
        } else {
          const diagnostic = sanitizeLogText(event.data).slice(0, 500);
          setMessage(`收到未知日志事件 ${event.event || "message"}：${diagnostic}`);
        }
      },
    }).catch((error: unknown) => {
      if (!controller.signal.aborted) {
        if (error instanceof ApiError && error.status === 403) {
          setLogs([]);
          lastSequence.current = 0;
          onAuthorizationRevoked(error);
          return;
        }
        setConnection("disconnected");
        setMessage("日志连接已断开，已加载内容仍然保留。");
      }
    });
    return () => controller.abort();
  }, [deploymentId, generation, onAuthorizationRevoked]);

  useEffect(() => {
    if (!following || !viewport.current) return;
    viewport.current.scrollTop = viewport.current.scrollHeight;
  }, [following, logs]);

  const sections = logs.reduce<Array<{ stage: string; logs: ReturnType<typeof DeploymentLogResponseFromJSON>[] }>>((groups, log) => {
    const stage = log.stage ?? "legacy";
    const last = groups.at(-1);
    if (last && last.stage === stage) {
      last.logs.push(log);
    } else {
      groups.push({ stage, logs: [log] });
    }
    return groups;
  }, []);
  const stageLabels: Record<string, string> = {
    prepare: "准备阶段（prepare）",
    release: "发布阶段（release）",
    legacy: "脚本阶段",
  };

  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(formatDeploymentLogs(logs));
      setCopyState("copied");
    } catch {
      setCopyState("failed");
    }
  }

  return <section className="log-workspace"><div className="log-toolbar"><div><strong>执行日志</strong><span className={`connection-state connection-state--${connection}`}>{connectionLabels[connection]}</span></div><div><Button disabled={logs.length === 0} onClick={() => void copyLogs()}>{copyState === "copied" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}{copyState === "copied" ? "已复制" : "复制日志"}</Button><Button onClick={() => setFollowing((value) => !value)}>{following ? "暂停跟随" : "恢复跟随"}</Button>{connection === "disconnected" ? <Button onClick={() => { setMessage(""); setGeneration((value) => value + 1); }}>重新连接</Button> : null}<Button onClick={() => { const node = viewport.current; if (node) node.scrollTop = node.scrollHeight; }}>跳到末尾</Button></div></div>{copyState === "failed" ? <p className="log-notice" role="status">复制失败，请选中日志内容后手动复制。</p> : null}{message ? <p className="log-notice" role="status">{message}</p> : null}<div className="log-viewport" ref={viewport} data-testid="deployment-log"><pre>{logs.length === 0 ? <span className="log-empty">等待脚本输出...</span> : sections.map((section) => <span className="log-section" key={section.stage}><span className="log-section-label">{stageLabels[section.stage] ?? section.stage}</span>{section.logs.map((log) => <span className={`log-line log-line--${log.stream}`} key={log.sequence}><i>{log.sequence}</i><b>{log.stream}</b><span>{log.content}{log.truncated ? " [已截断]" : ""}</span>{"\n"}</span>)}</span>)}</pre></div>{logs.length >= 1000 ? <p className="log-window-notice">为控制浏览器内存，仅显示最近 1000 条日志。</p> : null}</section>;
}
