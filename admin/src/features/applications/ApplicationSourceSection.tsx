import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { GitBranch, RefreshCw, ShieldCheck } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import type { GitRefDiscoveryResponse } from "../../api/generated/models/GitRefDiscoveryResponse";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { Field, Select, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import {
  applicationSourcesApi,
  gitCredentialsApi,
  sourceAgentsApi,
} from "./api";

const discoveryLabels: Record<string, string> = {
  queued: "等待 Agent 查询",
  running: "正在查询远程分支",
  succeeded: "查询完成",
  failed: "查询失败",
  expired: "结果已过期",
};

const discoveryErrorLabels: Record<string, string> = {
  git_authentication_failed: "Git 认证失败，请检查凭证是否已配置为只读 deploy key。",
  git_repository_unreachable: "仓库不可达，请检查地址和网络。",
  git_ref_discovery_timeout: "分支查询超时，请稍后重试。",
  secret_lease_failed: "Git 私钥租约获取失败，请检查凭证状态。",
};

interface SourceDraft {
  repositoryUrl: string;
  gitCredentialId: string;
  buildAgentId: string;
}

export function ApplicationSourceSection({ applicationId, isAdministrator, applicationActive }: { applicationId: string; isAdministrator: boolean; applicationActive: boolean }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<SourceDraft | null>(null);
  const [discovery, setDiscovery] = useState<GitRefDiscoveryResponse | null>(null);
  const [selectedBranch, setSelectedBranch] = useState("");
  const source = useQuery({
    queryKey: ["application-source", applicationId],
    queryFn: () => applicationSourcesApi.applicationSourceShow({ applicationId }),
    retry: false,
  });
  const credentials = useQuery({
    queryKey: ["git-credentials", "source-options"],
    queryFn: () => gitCredentialsApi.gitCredentialsList({ limit: 200 }),
    enabled: isAdministrator,
  });
  const agents = useQuery({
    queryKey: ["agents", "source-options"],
    queryFn: () => sourceAgentsApi.agentsList({ limit: 200 }),
    enabled: isAdministrator,
  });
  const sourceMissing = source.isError && source.error instanceof ApiError && source.error.status === 404;
  const form = draft ?? (source.data ? {
    repositoryUrl: source.data.repositoryUrl,
    gitCredentialId: source.data.gitCredentialId ?? "",
    buildAgentId: source.data.buildAgentId,
  } : null);
  const dirty = Boolean(form && (!source.data || (
    form.repositoryUrl !== source.data.repositoryUrl ||
    form.gitCredentialId !== (source.data.gitCredentialId ?? "") ||
    form.buildAgentId !== source.data.buildAgentId
  )));
  useUnsavedChanges(editing && (Boolean(form) && (!source.data || dirty)));

  const save = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !form) throw new Error("缺少必要的安全上下文");
    return applicationSourcesApi.applicationSourceSave({
      applicationId,
      xCSRFToken: auth.csrfToken,
      saveSourceRequest: {
        repositoryUrl: form.repositoryUrl.trim(),
        gitCredentialId: form.gitCredentialId || null,
        buildAgentId: form.buildAgentId,
        sourcePolicy: "branch",
        version: source.data?.version ?? undefined,
      },
    });
  }, onSuccess: (saved) => {
    queryClient.setQueryData(["application-source", applicationId], saved);
    setDraft({ repositoryUrl: saved.repositoryUrl, gitCredentialId: saved.gitCredentialId ?? "", buildAgentId: saved.buildAgentId });
    setDiscovery(null);
    setSelectedBranch("");
  } });
  const refresh = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return applicationSourcesApi.applicationSourceRefreshRefs({ applicationId, xCSRFToken: auth.csrfToken });
  }, onSuccess: (result) => { setDiscovery(result); setSelectedBranch(""); } });
  const fixBranch = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !source.data || !selectedBranch) throw new Error("请先选择分支");
    return applicationSourcesApi.applicationSourceSetBranch({
      applicationId,
      xCSRFToken: auth.csrfToken,
      setBranchRequest: { branch: selectedBranch, version: source.data.version },
    });
  }, onSuccess: (saved) => {
    queryClient.setQueryData(["application-source", applicationId], saved);
    setDraft(null);
    setDiscovery(null);
    setSelectedBranch("");
    setEditing(false);
  } });

  useEffect(() => {
    if (!discovery || !["queued", "running"].includes(discovery.status)) return;
    const timer = window.setTimeout(() => {
      void applicationSourcesApi.applicationSourceRefreshShow({ applicationId, refsQueryId: discovery.id })
        .then(setDiscovery)
        .catch(() => setDiscovery((current) => current ? { ...current, status: "failed", errorCode: "git_ref_discovery_failed" } : current));
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [applicationId, discovery]);

  function updateDraft(patch: Partial<SourceDraft>) {
    const base = form ?? { repositoryUrl: "", gitCredentialId: "", buildAgentId: "" };
    setDraft({ ...base, ...patch });
    setDiscovery(null);
    setSelectedBranch("");
  }

  async function submitSave(event: FormEvent) {
    event.preventDefault();
    await save.mutateAsync().catch(() => undefined);
  }

  const usableAgents = (agents.data?.items ?? []).filter((agent) => agent.status === "online" && (agent.protocolVersion ?? 0) >= 11);
  const usableCredentials = (credentials.data?.items ?? []).filter((credential) => credential.status === "active");
  const editForm = editing && form ? <form className="source-form" onSubmit={(event) => void submitSave(event)}>
    <Field label="仓库地址" className="form-span"><TextInput required value={form.repositoryUrl} onChange={(event) => updateDraft({ repositoryUrl: event.target.value })} placeholder="git@github.com:org/repo.git" /></Field>
    <Field label="Git 凭证"><Select required value={form.gitCredentialId} onChange={(event) => updateDraft({ gitCredentialId: event.target.value })}><option value="">公开仓库（无凭证）</option>{usableCredentials.map((credential) => <option key={credential.id} value={credential.id}>{credential.name}</option>)}</Select></Field>
    <Field label="构建节点"><Select required value={form.buildAgentId} onChange={(event) => updateDraft({ buildAgentId: event.target.value })}><option value="">选择在线节点</option>{usableAgents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name} · v{agent.agentVersion || "-"}</option>)}</Select></Field>
    {save.error ? <div className="form-span"><ApiErrorNotice error={toNotice(save.error)} /></div> : null}
    <div className="form-actions form-span"><Button type="button" disabled={save.isPending} onClick={() => { setEditing(false); setDraft(null); setDiscovery(null); setSelectedBranch(""); }}>取消编辑</Button><Button tone="primary" disabled={save.isPending || !dirty}>{save.isPending ? "正在保存..." : "保存来源"}</Button>{!dirty ? <Button type="button" disabled={refresh.isPending || !source.data} onClick={() => void refresh.mutate()}><RefreshCw aria-hidden="true" />{refresh.isPending ? "正在刷新..." : "刷新分支"}</Button> : null}</div>
    {discovery ? <div className="source-discovery form-span" aria-live="polite">
      <div className="section-heading"><div><h4>分支发现</h4><p>{discoveryLabels[discovery.status] ?? discovery.status}{discovery.errorCode ? ` · ${discoveryErrorLabels[discovery.errorCode] ?? discovery.errorCode}` : ""}</p></div></div>
      {discovery.status === "succeeded" ? <>
        {discovery.refs.length === 0 ? <p className="notice">远程仓库没有可部署分支。</p> : <div className="branch-fix-form">
          <Field label="固定分支"><Select required value={selectedBranch} onChange={(event) => setSelectedBranch(event.target.value)}><option value="">选择分支</option>{discovery.refs.map((ref) => <option key={ref.name} value={ref.name}>{ref.name} · {ref.sha.slice(0, 10)}</option>)}</Select></Field>
          {fixBranch.error ? <ApiErrorNotice error={toNotice(fixBranch.error)} /> : null}
          <div className="form-actions"><Button type="button" tone="primary" disabled={fixBranch.isPending || !selectedBranch || !source.data} onClick={() => void fixBranch.mutateAsync().catch(() => undefined)}><ShieldCheck aria-hidden="true" />{fixBranch.isPending ? "正在固定..." : "固定分支并完成配置"}</Button></div>
        </div>}
      </> : discovery.status === "failed" || discovery.status === "expired" ? <p className="notice notice--danger" role="alert">{discoveryErrorLabels[discovery.errorCode ?? ""] ?? "分支查询失败，请重新刷新。"}</p> : <PageState kind="loading" />}
    </div> : null}
  </form> : null;

  return <section className="detail-section">
    <div className="section-heading"><div><h3>Git 来源</h3><p>绑定仓库、构建节点与固定部署分支；普通用户只能查看已固定配置。</p></div>{isAdministrator && applicationActive && !editing ? <Button onClick={() => { setEditing(true); setDraft(null); }}><GitBranch aria-hidden="true" />配置来源</Button> : null}</div>
    {source.isLoading ? <PageState kind="loading" /> : source.isError && !sourceMissing ? <ApiErrorNotice error={toNotice(source.error)} /> : sourceMissing ? (!editing ? <div className="empty-inline"><p>应用尚未配置 Git 来源，两阶段部署目标需要先完成配置。</p>{isAdministrator && applicationActive ? <Button tone="primary" onClick={() => { setEditing(true); setDraft({ repositoryUrl: "", gitCredentialId: "", buildAgentId: usableAgents[0]?.id ?? "" }); }}>开始配置</Button> : null}</div> : editForm) : source.data ? <>
      {!editing ? <dl className="definition-grid"><div><dt>仓库地址</dt><dd><code>{source.data.repositoryUrl}</code></dd></div><div><dt>部署分支</dt><dd>{source.data.deploymentBranch ? <code>{source.data.deploymentBranch}</code> : <span className="text-muted">未固定</span>}</dd></div><div><dt>构建节点</dt><dd>{source.data.buildAgentName || source.data.buildAgentId}</dd></div><div><dt>Git 凭证</dt><dd>{source.data.gitCredentialName || "公开仓库"}</dd></div><div><dt>分支验证时间</dt><dd>{source.data.branchVerifiedAt ? new Date(source.data.branchVerifiedAt).toLocaleString("zh-CN") : "-"}</dd></div><div><dt>来源状态</dt><dd><span className={`status-badge status-badge--${source.data.status === "verified" ? "online" : "pending"}`}>{source.data.status === "verified" ? "已验证" : "草稿"}</span></dd></div></dl> : null}
      {editForm}
    </> : null}
  </section>;
}
