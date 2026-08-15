import { Activity, ChevronLeft, ChevronRight, Plus } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { Button } from "../../components/Button";
import { Select } from "../../components/form";
import { PageState } from "../../components/PageState";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { deploymentsApi } from "./api";
import { deploymentStatusLabel, deploymentStatusTone, formatDeploymentDuration } from "./status";

export function DeploymentsPage() {
  const [status, setStatus] = useState("all");
  const [pageIndex, setPageIndex] = useState(0);
  const deployments = useCursorCollection(["deployments"], (after) => deploymentsApi.list(after ?? undefined, 10));
  const pages = deployments.data?.pages ?? [];
  const currentItems = pages[pageIndex]?.items ?? [];
  const visible = status === "all" ? currentItems : currentItems.filter((item) => item.status === status);
  const canGoNext = pageIndex < pages.length - 1 || deployments.hasNextPage;

  async function goNext() {
    if (pageIndex < pages.length - 1) {
      setPageIndex((value) => value + 1);
      return;
    }
    if (!deployments.hasNextPage) return;
    const result = await deployments.fetchNextPage();
    if ((result.data?.pages.length ?? 0) > pageIndex + 1) setPageIndex((value) => value + 1);
  }

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>部署记录</h2><p>查看脚本执行状态、日志和最终结果。</p></div><Link className="button button--primary" to="/deployments/new"><Plus aria-hidden="true" />发起部署</Link></div>
    <div className="filter-bar"><label>状态<Select value={status} onChange={(event) => { setStatus(event.target.value); setPageIndex(0); }}><option value="all">全部状态</option><option value="queued">排队中</option><option value="running">运行中</option><option value="failed">失败</option><option value="interrupted">执行中断</option><option value="succeeded">成功</option><option value="canceled">已取消</option></Select></label></div>
    {deployments.isLoading ? <PageState kind="loading" /> : deployments.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(deployments.error)} /><Button onClick={() => void deployments.refetch()}>重试</Button></div> : deployments.items.length === 0 ? <PageState kind="empty" /> : <>{visible.length === 0 ? <div className="empty-inline">当前页没有匹配该状态的部署。</div> : <div className="data-table-wrap"><table className="data-table data-table--priority deployment-table"><thead><tr><th>部署</th><th className="table-column--secondary">目标</th><th>状态</th><th className="table-column--secondary">阶段</th><th className="table-column--secondary">创建时间</th><th className="table-column--secondary">耗时</th><th></th></tr></thead><tbody>{visible.map((deployment) => <tr key={deployment.id}><td><Activity aria-hidden="true" /><strong><code>{deployment.id}</code></strong></td><td className="table-column--secondary"><code>{deployment.targetId}</code></td><td><span className={`status-badge status-badge--${deploymentStatusTone(deployment.status)}`}>{deploymentStatusLabel(deployment.status)}</span></td><td className="table-column--secondary">{deployment.phase}</td><td className="table-column--secondary">{new Date(deployment.createdAt).toLocaleString()}</td><td className="table-column--secondary">{deployment.finishedAt ? formatDeploymentDuration(deployment.queuedAt, deployment.finishedAt) : "进行中"}</td><td><Link className="text-link" to={`/deployments/${deployment.id}`}>查看</Link></td></tr>)}</tbody></table></div>}<nav className="pagination-actions" aria-label="部署记录分页"><Button aria-label="上一页" disabled={pageIndex === 0} onClick={() => setPageIndex((value) => Math.max(0, value - 1))}><ChevronLeft aria-hidden="true" />上一页</Button><span className="pagination-current">第 {pageIndex + 1} 页</span><Button aria-label="下一页" disabled={!canGoNext || deployments.isFetchingNextPage} onClick={() => void goNext()}>{deployments.isFetchingNextPage ? "正在加载..." : <>下一页<ChevronRight aria-hidden="true" /></>}</Button></nav></>}
  </section>;
}
