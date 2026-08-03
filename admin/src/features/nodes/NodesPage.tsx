import { useQuery } from "@tanstack/react-query";
import { Bot, Server } from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { agentsApi } from "../agents/api";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { nodesApi } from "./api";

export function NodesPage() {
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const nodes = useCursorCollection(["nodes"], (after) => nodesApi.nodesList({ limit: 50, after: after ?? undefined }));
  const agents = useQuery({ queryKey: ["agents", "node-links"], queryFn: () => agentsApi.agentsList({ limit: 200 }), enabled: isAdministrator });
  const agentByNode = new Map(agents.data?.items.map((agent) => [agent.nodeId, agent]));

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>节点</h2><p>{isAdministrator ? "节点由 Agent 身份创建，在线状态来自当前控制连接。" : "查看已授权应用关联的节点与在线状态。"}</p></div>{isAdministrator ? <Link className="button button--primary" to="/agents"><Bot aria-hidden="true" />管理 Agent</Link> : null}</div>
    {nodes.isLoading ? <PageState kind="loading" /> : nodes.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(nodes.error)} /><Button onClick={() => void nodes.refetch()}>重试</Button></div> : nodes.items.length === 0 ? <PageState kind="empty" /> : <><div className="data-table-wrap"><table className="data-table"><thead><tr><th>节点</th><th>状态</th><th>工作目录</th>{isAdministrator ? <th>Agent</th> : null}<th></th></tr></thead><tbody>{nodes.items.map((node) => { const agent = agentByNode.get(node.id); return <tr key={node.id}><td><Server aria-hidden="true" /><strong>{node.name}</strong></td><td><span className={`status-badge status-badge--${node.status === "online" ? "online" : "offline"}`}>{node.status === "online" ? "在线" : "离线"}</span></td><td><code>{node.workRoot || "尚未上报"}</code></td>{isAdministrator ? <td>{agent ? <Link className="text-link" to={`/agents/${agent.id}`}>{agent.name}</Link> : "未关联"}</td> : null}<td><Link className="text-link" to={`/nodes/${node.id}`}>查看</Link></td></tr>; })}</tbody></table></div>{nodes.hasNextPage ? <div className="pagination-actions"><Button disabled={nodes.isFetchingNextPage} onClick={() => void nodes.fetchNextPage()}>加载更多</Button></div> : null}</>}
  </section>;
}

export function statusLabel(status: string) { return status === "online" ? "在线" : "离线"; }
