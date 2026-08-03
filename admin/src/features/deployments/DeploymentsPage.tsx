import { Activity, Plus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { deploymentsApi } from "./api";
import { deploymentStatusLabel, deploymentStatusTone } from "./status";

export function DeploymentsPage() {
  const [status, setStatus] = useState("all");
  const deployments = useCursorCollection(["deployments"], (after) => deploymentsApi.list(after ?? undefined));
  const visible = status === "all" ? deployments.items : deployments.items.filter((item) => item.status === status);
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>部署记录</h2><p>查看脚本执行状态、日志和最终结果。</p></div><Link className="button button--primary" to="/deployments/new"><Plus aria-hidden="true" />发起部署</Link></div>
    <div className="filter-bar"><label>状态<select value={status} onChange={(event) => setStatus(event.target.value)}><option value="all">全部状态</option><option value="queued">排队中</option><option value="running">运行中</option><option value="failed">失败</option><option value="interrupted">执行中断</option><option value="succeeded">成功</option><option value="canceled">已取消</option></select></label></div>
    {deployments.isLoading ? <PageState kind="loading" /> : deployments.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(deployments.error)} /><Button onClick={() => void deployments.refetch()}>重试</Button></div> : deployments.items.length === 0 ? <PageState kind="empty" /> : visible.length === 0 ? <div className="empty-inline">没有匹配当前状态的部署。</div> : <><div className="data-table-wrap"><table className="data-table deployment-table"><thead><tr><th>部署</th><th>目标</th><th>状态</th><th>阶段</th><th>创建时间</th><th></th></tr></thead><tbody>{visible.map((deployment) => <tr key={deployment.id}><td><Activity aria-hidden="true" /><strong><code>{deployment.id}</code></strong></td><td><code>{deployment.targetId}</code></td><td><span className={`status-badge status-badge--${deploymentStatusTone(deployment.status)}`}>{deploymentStatusLabel(deployment.status)}</span></td><td>{deployment.phase}</td><td>{new Date(deployment.createdAt).toLocaleString()}</td><td><Link className="text-link" to={`/deployments/${deployment.id}`}>查看</Link></td></tr>)}</tbody></table></div>{deployments.hasNextPage ? <div className="pagination-actions"><Button disabled={deployments.isFetchingNextPage} onClick={() => void deployments.fetchNextPage()}>{deployments.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}</>}
  </section>;
}
