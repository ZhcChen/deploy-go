import { useEffect, useRef, useState } from "react";
import { DeploymentLogResponseFromJSON } from "../../api/generated/models/DeploymentLogResponse";
import { Button } from "../../components/Button";
import { streamSse, type SseConnectionState } from "../../api/sse-client";
import { appendDeploymentLog, sanitizeLogText } from "./log-store";

const connectionLabels: Record<SseConnectionState | "disconnected" | "revoked", string> = {
  connecting: "连接中",
  open: "实时连接",
  reconnecting: "正在重连",
  ended: "已结束",
  disconnected: "连接已断开",
  revoked: "访问授权已失效",
};

export function DeploymentLogPanel({ deploymentId, onTerminal }: { deploymentId: string; onTerminal(): void }) {
  const [logs, setLogs] = useState<ReturnType<typeof DeploymentLogResponseFromJSON>[]>([]);
  const [connection, setConnection] = useState<SseConnectionState | "disconnected" | "revoked">("connecting");
  const [message, setMessage] = useState("");
  const [following, setFollowing] = useState(true);
  const [generation, setGeneration] = useState(0);
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
          setConnection("revoked");
          setMessage("日志访问授权已经失效，已停止接收新内容。");
        } else if (event.event === "terminal") {
          setConnection("ended");
          terminalCallback.current();
        } else {
          const diagnostic = sanitizeLogText(event.data).slice(0, 500);
          setMessage(`收到未知日志事件 ${event.event || "message"}：${diagnostic}`);
        }
      },
    }).catch(() => {
      if (!controller.signal.aborted) {
        setConnection("disconnected");
        setMessage("日志连接已断开，已加载内容仍然保留。");
      }
    });
    return () => controller.abort();
  }, [deploymentId, generation]);

  useEffect(() => {
    if (!following || !viewport.current) return;
    viewport.current.scrollTop = viewport.current.scrollHeight;
  }, [following, logs]);

  return <section className="log-workspace"><div className="log-toolbar"><div><strong>执行日志</strong><span className={`connection-state connection-state--${connection}`}>{connectionLabels[connection]}</span></div><div><Button onClick={() => setFollowing((value) => !value)}>{following ? "暂停跟随" : "恢复跟随"}</Button>{connection === "disconnected" ? <Button onClick={() => { setMessage(""); setGeneration((value) => value + 1); }}>重新连接</Button> : null}<Button onClick={() => { const node = viewport.current; if (node) node.scrollTop = node.scrollHeight; }}>跳到末尾</Button></div></div>{message ? <p className="log-notice" role="status">{message}</p> : null}<div className="log-viewport" ref={viewport} data-testid="deployment-log"><pre>{logs.length === 0 ? <span className="log-empty">等待脚本输出...</span> : logs.map((log) => <span className={`log-line log-line--${log.stream}`} key={log.sequence}><i>{log.sequence}</i><b>{log.stream}</b><span>{log.content}{log.truncated ? " [已截断]" : ""}</span>{"\n"}</span>)}</pre></div>{logs.length >= 1000 ? <p className="log-window-notice">为控制浏览器内存，仅显示最近 1000 条日志。</p> : null}</section>;
}
