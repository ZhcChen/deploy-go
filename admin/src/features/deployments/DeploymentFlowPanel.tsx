import { useQuery } from "@tanstack/react-query";
import { Check, ChevronDown, Circle, LoaderCircle, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { DeploymentResponse } from "../../api/generated/models/DeploymentResponse";
import { Button } from "../../components/Button";
import { deploymentsApi } from "./api";
import { buildDeploymentFlow, type FlowStage, type FlowStatus } from "./deployment-flow";
import { isTerminalDeployment } from "./status";

export function DeploymentFlowPanel({ deployment, onViewLogs }: { deployment: DeploymentResponse; onViewLogs(stage: "prepare" | "release"): void }) {
  const terminal = isTerminalDeployment(deployment.status);
  const events = useQuery({
    queryKey: ["deployment-events", deployment.id],
    queryFn: () => deploymentsApi.events(deployment.id),
    refetchInterval: terminal ? false : 2000,
    refetchOnWindowFocus: !terminal,
  });
  // 部署刚变成终态时再刷新一次事件。最后几条流程事件通常和终态结果几乎同时落库，常常晚于上一次轮询；
  // 流程图的阶段状态又以事件为准，而终态之后不再轮询，不补这一次，页面顶部已经是“成功”，
  // 流程图的最后一个阶段还会停在“执行中”。首次加载就是终态的部署不会重复请求。
  const tracked = useRef({ id: deployment.id, terminal });
  const refetchEvents = events.refetch;
  useEffect(() => {
    const previous = tracked.current;
    if (previous.id === deployment.id && terminal && !previous.terminal) void refetchEvents();
    tracked.current = { id: deployment.id, terminal };
  }, [deployment.id, terminal, refetchEvents]);
  const stages = useMemo(() => buildDeploymentFlow(deployment, events.data ?? []), [deployment, events.data]);
  const failedStage = stages.find((stage) => stage.status === "failed")?.stage;
  const [selectedStage, setSelectedStage] = useState<string | null>(null);
  const expanded = selectedStage ?? failedStage ?? null;

  return <section className="deployment-flow-panel" aria-label="部署流程">
    <ol className="deployment-flow-track">{stages.map((stage) => <li key={stage.stage} className={`deployment-flow-stage deployment-flow-stage--${stage.status}`}>
      <button type="button" aria-expanded={expanded === stage.stage} aria-controls={`flow-stage-${stage.stage}`} onClick={() => setSelectedStage(expanded === stage.stage ? "" : stage.stage)}>
        <FlowStatusIcon status={stage.status} />
        <span><strong>{stage.label}</strong><small>{stage.statusLabel}</small></span>
        <ChevronDown className="deployment-flow-chevron" aria-hidden="true" />
      </button>
    </li>)}</ol>
    {stages.map((stage) => expanded === stage.stage ? <StageDetails key={stage.stage} stage={stage} onViewLogs={onViewLogs} /> : null)}
    {!deployment.protocolComplete ? <p className="flow-protocol-note">流程仅展示平台可以证明的结构化事件；该部署的事件协议不完整。</p> : null}
  </section>;
}

function StageDetails({ stage, onViewLogs }: { stage: FlowStage; onViewLogs(stage: "prepare" | "release"): void }) {
  return <div className="deployment-flow-details" id={`flow-stage-${stage.stage}`}>
    <div className="section-heading"><div><h3>{stage.label}</h3><p>{stage.items.length > 0 ? `${stage.items.length} 个模块或步骤` : "暂无结构化步骤"}</p></div>{stage.status === "failed" ? <Button onClick={() => onViewLogs(stage.logStage)}>查看相关日志</Button> : null}</div>
    {stage.items.length > 0 ? <ul>{stage.items.map((item) => <li key={item.id}><FlowStatusIcon status={item.status} /><span><strong>{item.label}</strong><small>{item.statusLabel}</small></span></li>)}</ul> : <p className="muted">该阶段尚未产生可展示的模块或步骤事件。</p>}
  </div>;
}

function FlowStatusIcon({ status }: { status: FlowStatus }) {
  const Icon = status === "succeeded" ? Check : status === "failed" ? X : status === "running" ? LoaderCircle : Circle;
  return <span className={`flow-status-icon flow-status-icon--${status}`} aria-label={{ idle: "未执行", running: "执行中", succeeded: "成功", failed: "失败" }[status]}><Icon aria-hidden="true" /></span>;
}
