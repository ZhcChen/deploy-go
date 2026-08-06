import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, Plus } from "lucide-react";
import { Link } from "react-router-dom";
import type { DeploymentResponse } from "../../api/generated/models/DeploymentResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { applicationsApi, deploymentTargetsApi } from "../applications/api";
import { deploymentsApi } from "../deployments/api";
import { deploymentStatusLabel, deploymentStatusTone } from "../deployments/status";
import { nodesApi } from "../nodes/api";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";

const ACTIVE_STATUSES = ["queued", "running", "canceling"];

function activityDotTone(status: string) {
  if (status === "failed" || status === "interrupted") return "danger";
  if (ACTIVE_STATUSES.includes(status)) return "pending";
  return "muted";
}

export function OverviewPage() {
  const deployments = useQuery({
    queryKey: ["overview", "deployments"],
    queryFn: () => deploymentsApi.list(),
  });
  const nodes = useQuery({
    queryKey: ["overview", "nodes"],
    queryFn: () => nodesApi.nodesList({ limit: 200 }),
  });
  const applications = useQuery({
    queryKey: ["overview", "applications"],
    queryFn: () => applicationsApi.applicationsList({ limit: 200 }),
  });

  const recentDeployments = deployments.data?.items.slice(0, 5) ?? [];
  const targetIds = recentDeployments.map((item) => item.targetId);
  const targets = useQuery({
    queryKey: ["overview", "targets", targetIds],
    enabled: targetIds.length > 0,
    queryFn: async () => {
      const results = await Promise.allSettled(
        targetIds.map((id) => deploymentTargetsApi.deploymentTargetsShow({ id })),
      );
      return results.flatMap((result) => (result.status === "fulfilled" ? [result.value] : []));
    },
  });

  const queries = [deployments, nodes, applications, targets];
  const isLoading = queries.some((query) => query.isPending && query.isEnabled);
  const firstError = queries.find((query) => query.isError);
  const items = deployments.data?.items ?? [];
  const nodeItems = nodes.data?.items ?? [];
  const appById = new Map(applications.data?.items.map((app) => [app.id, app]) ?? []);
  const nodeById = new Map(nodeItems.map((node) => [node.id, node]));
  const targetById = new Map((targets.data ?? []).map((target) => [target.id, target]));

  const running = items.filter((item) => ACTIVE_STATUSES.includes(item.status)).length;
  const failed = items.filter((item) => item.status === "failed").length;
  const offline = nodeItems.filter((node) => node.status === "offline").length;
  const offlineNodes = nodeItems.filter((node) => node.status === "offline").slice(0, 3);
  const failedDeployments = items.filter((item) => item.status === "failed").slice(0, 3);

  function deploymentTitle(deployment: DeploymentResponse) {
    const target = targetById.get(deployment.targetId);
    const app = target ? appById.get(target.applicationId) : undefined;
    return app?.name ?? deployment.targetId;
  }

  return <section className="workspace">
    <div className="workspace-heading">
      <div><h2>概览</h2><p>最近的部署运行状态与需要关注的异常。</p></div>
      <Link className="button button--primary" to="/deployments/new"><Plus aria-hidden="true" />发起部署</Link>
    </div>
    {isLoading ? <PageState kind="loading" /> : firstError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(firstError.error)} /><Button onClick={() => void Promise.all(queries.map((query) => query.refetch()))}>重试</Button></div> : nodeItems.length === 0 ? <PageState kind="empty" /> : <>
      <div className="overview-metrics" aria-label="运行摘要">
        <div className="overview-metric"><span>运行中的部署</span><strong>{running}<small>任务</small></strong></div>
        <div className="overview-metric"><span>失败待处理</span><strong>{failed}<small>记录</small></strong></div>
        <div className="overview-metric"><span>异常节点</span><strong>{offline}<small>/ {nodeItems.length}</small></strong></div>
        <div className="overview-metric"><span>已配置应用</span><strong>{applications.data?.items.length ?? 0}<small>个</small></strong></div>
      </div>
      <div className="overview-grid">
        <section aria-label="最近活动">
          <div className="section-heading"><div><h3>最近活动</h3><p>应用脚本的执行结果</p></div><Link className="muted-link" to="/deployments">查看全部</Link></div>
          {recentDeployments.length === 0 ? <div className="empty-inline">还没有部署记录。</div> : <div className="activity-list">{recentDeployments.map((deployment) => { const target = targetById.get(deployment.targetId); return <Link className="activity-row" key={deployment.id} to={`/deployments/${deployment.id}`}><span className={`activity-row__dot activity-row__dot--${activityDotTone(deployment.status)}`} /><div><strong>{deploymentTitle(deployment)}</strong><span className="muted">{target && nodeById.get(target.nodeId) ? `${nodeById.get(target.nodeId)!.name} · ` : ""}{new Date(deployment.createdAt).toLocaleString()}</span></div><span className={`status-badge status-badge--${deploymentStatusTone(deployment.status)}`}>{deploymentStatusLabel(deployment.status)}</span></Link>; })}</div>}
        </section>
        <aside aria-label="需要关注">
          <div className="section-heading"><div><h3>需要关注</h3><p>影响部署条件的异常</p></div></div>
          {offlineNodes.length === 0 && failedDeployments.length === 0 ? <div className="empty-inline">当前没有需要关注的问题。</div> : <div className="alert-list">
            {offlineNodes.map((node) => <Link className="alert-item" key={node.id} to={`/nodes/${node.id}`}><strong><AlertTriangle aria-hidden="true" />节点 {node.name} 离线</strong><p>该节点当前不能执行部署，协同程序重连后自动恢复在线。</p></Link>)}
            {failedDeployments.map((deployment) => <Link className="alert-item" key={deployment.id} to={`/deployments/${deployment.id}`}><strong><AlertTriangle aria-hidden="true" />{deploymentTitle(deployment)} 部署失败</strong><p>{deployment.resultSummary || "脚本执行未成功，请查看日志确认原因。"}</p></Link>)}
          </div>}
        </aside>
      </div>
    </>}
  </section>;
}
