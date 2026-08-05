import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, RefreshCw } from "lucide-react";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { agentsApi } from "./api";

export function AgentReleasesPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const releases = useQuery({
    queryKey: ["agent-releases"],
    queryFn: () => agentsApi.agentReleasesList(),
  });
  const remove = useMutation({
    mutationFn: async (version: string) => {
      if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
      if (!window.confirm(`清理 Agent ${version} 后，该版本将无法再被节点下载，确定继续吗？`)) return;
      await agentsApi.agentReleasesDelete({ version, xCSRFToken: auth.csrfToken });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["agent-releases"] });
    },
  });

  if (releases.isLoading) return <PageState kind="loading" />;
  if (releases.isError || !releases.data) {
    return <div className="state-with-action"><ApiErrorNotice error={toNotice(releases.error)} /><Button onClick={() => void releases.refetch()}>重试</Button></div>;
  }

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>Agent 版本</h2><p>查看本服务已同步的 Agent 发布物，历史版本可在确认后清理。新版本由部署端同步脚本下载并放入发布目录。</p></div><Button onClick={() => void releases.refetch()}><RefreshCw aria-hidden="true" />刷新</Button></div>
    {remove.error ? <ApiErrorNotice error={toNotice(remove.error)} /> : null}
    {releases.data.items.length === 0 ? <PageState kind="empty" /> : <div className="data-table-wrap"><table className="data-table"><thead><tr><th>版本</th><th>状态</th><th>控制协议</th><th></th></tr></thead><tbody>{releases.data.items.map((release) => {
      const active = release.version === releases.data!.currentVersion;
      return <tr key={release.version}><td><code>{release.version}</code></td><td><span className={`status-badge status-badge--${active ? "online" : "offline"}`}>{active ? "当前版本" : "历史版本"}</span></td><td><code>{release.protocolMinimum} - {release.protocolMaximum}</code></td><td><Button tone="danger" disabled={active || remove.isPending} onClick={() => void remove.mutateAsync(release.version).catch(() => undefined)}><Archive aria-hidden="true" />{remove.isPending ? "清理中..." : "清理"}</Button></td></tr>;
    })}</tbody></table></div>}
  </section>;
}
