import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderGit2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { Field, Select, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import {
  applicationWorkspaceSourcesApi,
  sourceAgentsApi,
} from "./api";

interface WorkspaceSourceDraft {
  buildAgentId: string;
  workspacePath: string;
}

export function WorkspaceSourceSection({ applicationId, isAdministrator, applicationActive }: { applicationId: string; isAdministrator: boolean; applicationActive: boolean }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<WorkspaceSourceDraft | null>(null);
  const source = useQuery({
    queryKey: ["application-workspace-source", applicationId],
    queryFn: () => applicationWorkspaceSourcesApi.applicationWorkspaceSourceShow({ applicationId }),
    retry: false,
  });
  const agents = useQuery({
    queryKey: ["agents", "workspace-source-options"],
    queryFn: () => sourceAgentsApi.agentsList({ limit: 200 }),
    enabled: isAdministrator,
  });
  const sourceMissing = source.isError && source.error instanceof ApiError && source.error.status === 404;
  const form = draft ?? (source.data ? {
    buildAgentId: source.data.buildAgentId,
    workspacePath: source.data.workspacePath,
  } : null);
  const dirty = Boolean(form && (!source.data || (
    form.buildAgentId !== source.data.buildAgentId ||
    form.workspacePath !== source.data.workspacePath
  )));
  useUnsavedChanges(editing && Boolean(form) && (!source.data || dirty));

  const save = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !form) throw new Error("缺少必要的安全上下文");
    return applicationWorkspaceSourcesApi.applicationWorkspaceSourceSave({
      applicationId,
      xCSRFToken: auth.csrfToken,
      saveWorkspaceSourceRequest: {
        buildAgentId: form.buildAgentId,
        workspacePath: form.workspacePath.trim(),
        version: source.data?.version ?? undefined,
      },
    });
  }, onSuccess: (saved) => {
    queryClient.setQueryData(["application-workspace-source", applicationId], saved);
    setDraft({ buildAgentId: saved.buildAgentId, workspacePath: saved.workspacePath });
    setEditing(false);
  } });

  const usableAgents = (agents.data?.items ?? []).filter((agent) => agent.status === "online" && (agent.protocolVersion ?? 0) >= 14);
  const editForm = editing && form ? <form className="source-form" onSubmit={(event: FormEvent) => { event.preventDefault(); void save.mutateAsync().catch(() => undefined); }}>
    <Field label="构建节点" className="form-span"><Select required value={form.buildAgentId} onChange={(event) => setDraft({ ...form, buildAgentId: event.target.value })}><option value="">选择在线节点</option>{usableAgents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name} · v{agent.agentVersion || "-"}</option>)}</Select></Field>
    <Field label="工作区路径" hint="构建 Agent 上的固定本地目录，prepare 前会先快照到任务 staging。" className="form-span"><TextInput required value={form.workspacePath} onChange={(event) => setDraft({ ...form, workspacePath: event.target.value })} placeholder="/srv/workspaces/clickhouse" /></Field>
    {save.error ? <div className="form-span"><ApiErrorNotice error={toNotice(save.error)} /></div> : null}
    <div className="form-actions form-span"><Button type="button" disabled={save.isPending} onClick={() => { setEditing(false); setDraft(null); }}>取消编辑</Button><Button tone="primary" disabled={save.isPending || !dirty}>{save.isPending ? "正在保存..." : "保存工作区来源"}</Button></div>
  </form> : null;

  return <section className="detail-section">
    <div className="section-heading"><div><h3>本地工作区来源</h3><p>脚本两阶段（two_stage_script）模式不依赖 Git，prepare 在构建 Agent 的固定工作区执行。</p></div>{isAdministrator && applicationActive && !editing && source.data ? <Button onClick={() => { setEditing(true); setDraft(null); }}><FolderGit2 aria-hidden="true" />配置工作区来源</Button> : null}</div>
    {source.isLoading ? <PageState kind="loading" /> : source.isError && !sourceMissing ? <ApiErrorNotice error={toNotice(source.error)} /> : sourceMissing ? (!editing ? <div className="empty-inline"><p>应用尚未配置本地工作区来源。</p>{isAdministrator && applicationActive ? <Button tone="primary" onClick={() => { setEditing(true); setDraft({ buildAgentId: usableAgents[0]?.id ?? "", workspacePath: "" }); }}>开始配置工作区</Button> : null}</div> : editForm) : source.data ? <>
      {!editing ? <dl className="definition-grid"><div><dt>构建节点</dt><dd>{source.data.buildAgentName || source.data.buildAgentId}</dd></div><div><dt>工作区路径</dt><dd><code>{source.data.workspacePath}</code></dd></div><div><dt>工作区版本</dt><dd><code>v{source.data.workspaceVersion}</code></dd></div><div><dt>更新时间</dt><dd>{new Date(source.data.updatedAt).toLocaleString("zh-CN")}</dd></div><div><dt>状态</dt><dd><span className={`status-badge status-badge--${source.data.status === "verified" ? "online" : "pending"}`}>{source.data.status === "verified" ? "已验证" : "草稿"}</span></dd></div></dl> : null}
      {editForm}
    </> : null}
  </section>;
}
