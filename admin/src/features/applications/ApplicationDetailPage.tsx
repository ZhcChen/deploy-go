import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Play, Plus, RefreshCw, Server, ShieldCheck } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import type { DeploymentTargetResponse } from "../../api/generated/models/DeploymentTargetResponse";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field, Select, TextArea, TextInput } from "../../components/form";
import { AGENT_ENVIRONMENTS, environmentLabel } from "../agents/environments";
import { PageState } from "../../components/PageState";
import { executionModeLabel, privilegedReleaseLabel } from "../targets/labels";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { TargetEditor } from "../targets/TargetEditor";
import { applicationNodesApi, applicationsApi, deploymentTargetsApi, runtimeProbeApi } from "./api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { LIST_PAGE_SIZE } from "../shared/pagination";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { ApplicationSourceSection } from "./ApplicationSourceSection";
import { WorkspaceSourceSection } from "./WorkspaceSourceSection";
import { ApplicationEnvSection } from "../application-envs/ApplicationEnvSection";
import { ApplicationConfigSection } from "../application-configs/ApplicationConfigSection";
import { formatBytesPair, formatPercent, NodeMetric } from "../nodes/NodeMetric";
import { moduleDefaults, moduleOptions } from "../deployments/ParameterEditor";
import { applicationRuntimeBadge, applicationRuntimeState } from "./runtimeBadge";
import { TagPickerField } from "./TagPicker";

type ApplicationDetailView = "overview" | "sources" | "env" | "config" | "targets";

const RUNTIME_PROBE_POLL_INTERVAL_MS = 1_500;
const RUNTIME_PROBE_TIMEOUT_MS = 20_000;

const APPLICATION_TYPE_OPTIONS = [
  { type: "binary", version: "1", label: "普通二进制 v1" },
  { type: "redis", version: "7", label: "Redis v7" },
  { type: "valkey", version: "9", label: "Valkey v9" },
  { type: "postgres", version: "16", label: "PostgreSQL v16" },
  { type: "postgres", version: "18", label: "PostgreSQL v18" },
] as const;

function applicationTypeLabel(appType: string, typeVersion: string) {
  const option = APPLICATION_TYPE_OPTIONS.find((item) => item.type === appType && item.version === typeVersion);
  return option?.label ?? `${appType} v${typeVersion}`;
}

export function ApplicationDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const isAdministrator = auth.user?.identity === "administrator";
  const [editing, setEditing] = useState(false);
  const [editingContract, setEditingContract] = useState(false);
  const [probing, setProbing] = useState(false);
  const [addingTarget, setAddingTarget] = useState(false);
  const app = useQuery({
    queryKey: ["application", id],
    queryFn: () => applicationsApi.applicationsShow({ id }),
  });
  const tagOptions = useQuery({ queryKey: ["application-tags"], queryFn: () => applicationsApi.applicationTagsList() });
  const availableTags = tagOptions.data?.tags ?? [];
  const targets = useCursorCollection(["deployment-targets", id], (after) => deploymentTargetsApi.deploymentTargetsList({ applicationId: id, limit: LIST_PAGE_SIZE, after: after ?? undefined }));
  const nodes = useCursorCollection(["nodes", "target-options"], (after) => applicationNodesApi.nodesList({ limit: 200, after: after ?? undefined }));
  const nodeById = new Map(nodes.items.map((node) => [node.id, node]));
  const [name, setName] = useState<string | null>(null);
  const [slug, setSlug] = useState<string | null>(null);
  const [description, setDescription] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<string | null>(null);
  const [appType, setAppType] = useState<string | null>(null);
  const [typeVersion, setTypeVersion] = useState<string | null>(null);
  const [tags, setTags] = useState<string[] | null>(null);
  const [parameterSchema, setParameterSchema] = useState<string | null>(null);
  const [verificationConfig, setVerificationConfig] = useState<string | null>(null);
  const [contractError, setContractError] = useState<string | null>(null);
  const metadataDirty = editing && (name !== null || slug !== null || description !== null || environment !== null || appType !== null || typeVersion !== null || tags !== null);
  const contractDirty = editingContract && (parameterSchema !== null || verificationConfig !== null);
  useUnsavedChanges(metadataDirty || contractDirty);
  const update = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdate({ id, xCSRFToken: auth.csrfToken, saveApplicationRequest: { name: (name ?? app.data.name).trim(), slug: (slug ?? app.data.slug).trim(), description: (description ?? app.data.description).trim(), appType: appType ?? app.data.appType, typeVersion: typeVersion ?? app.data.typeVersion, environment: (environment ?? app.data.environment), tags: tags ?? app.data.tags ?? [], parameterSchema: app.data.parameterSchema ?? {}, verificationConfig: app.data.verificationConfig ?? {}, version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); void queryClient.invalidateQueries({ queryKey: ["application-tags"] }); setEditing(false); discardMetadataDraft(); } });
  const contractUpdate = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdate({ id, xCSRFToken: auth.csrfToken, saveApplicationRequest: { name: app.data.name, slug: app.data.slug, description: app.data.description, appType: app.data.appType, typeVersion: app.data.typeVersion, environment: app.data.environment, tags: app.data.tags ?? [], parameterSchema: parseJsonObject(parameterSchema ?? JSON.stringify(app.data.parameterSchema ?? {}, null, 2), "参数 JSON Schema"), verificationConfig: parseJsonObject(verificationConfig ?? JSON.stringify(app.data.verificationConfig ?? {}, null, 2), "部署后验证配置"), version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); setEditingContract(false); discardContractDraft(); } });
  const status = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdateStatus({ id, xCSRFToken: auth.csrfToken, applicationStatusRequest: { status: app.data.status === "active" ? "archived" : "active", version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); } });
  const runtimeProbe = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
    return runtimeProbeApi.applicationsRuntimeProbesRequest({ xCSRFToken: auth.csrfToken, runtimeProbeBatchRequest: { applicationIds: [id] } });
  }, onSuccess: async (response) => {
    await queryClient.invalidateQueries({ queryKey: ["application", id] });
    if (!response.items.some((item) => item.status === "queued" || item.status === "in_progress")) return;
    setProbing(true);
    const deadline = Date.now() + RUNTIME_PROBE_TIMEOUT_MS;
    while (Date.now() < deadline) {
      await delay(RUNTIME_PROBE_POLL_INTERVAL_MS);
      const latest = await queryClient.fetchQuery({ queryKey: ["application", id], queryFn: () => applicationsApi.applicationsShow({ id }) });
      if (latest.runtimeProbeStatus === "succeeded" || latest.runtimeProbeStatus === "failed") break;
    }
    setProbing(false);
  } });
  function discardMetadataDraft() {
    setName(null); setSlug(null); setDescription(null); setEnvironment(null); setAppType(null); setTypeVersion(null); setTags(null);
  }
  function discardContractDraft() {
    setParameterSchema(null); setVerificationConfig(null); setContractError(null);
  }
  async function submit(event: FormEvent) {
    event.preventDefault();
    await update.mutateAsync().catch(() => undefined);
  }
  async function submitContract(event: FormEvent) {
    event.preventDefault();
    setContractError(null);
    try {
      parseJsonObject(parameterSchema ?? JSON.stringify(app.data?.parameterSchema ?? {}, null, 2), "参数 JSON Schema");
      parseJsonObject(verificationConfig ?? JSON.stringify(app.data?.verificationConfig ?? {}, null, 2), "部署后验证配置");
    } catch (error) {
      setContractError(error instanceof Error ? error.message : "部署契约 JSON 格式不正确");
      return;
    }
    await contractUpdate.mutateAsync().catch(() => undefined);
  }
  function changeStatus() {
    if (app.data?.status === "active" && !window.confirm("归档后将阻止创建和执行新的部署目标，确定继续吗？")) return;
    status.mutate();
  }
  if (app.isLoading) return <PageState kind="loading" />;
  if (app.isError || !app.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(app.error)} /><Link className="button button--default" to="/apps">返回应用</Link></div>;
  const requestedView = searchParams.get("view");
  const view: ApplicationDetailView = requestedView === "sources" || requestedView === "env" || requestedView === "config" || requestedView === "targets" ? requestedView : "overview";
  const selectView = (next: ApplicationDetailView) => {
    const params = new URLSearchParams(searchParams);
    if (next === "overview") params.delete("view");
    else params.set("view", next);
    setSearchParams(params);
  };
  const runtime = applicationRuntimeState(app.data);
  const modules = moduleOptions(app.data.parameterSchema);
  const defaultModules = moduleDefaults(app.data.parameterSchema);
  const enabledTargets = targets.items.filter((target) => target.status === "active").length;
  return <section className="workspace detail-page">
    <BackLink to="/apps" parentLabel="应用列表" />
    <div className="detail-title"><div><h2>{app.data.name}</h2><p><code>{app.data.slug}</code> · {app.data.description || "暂无说明"}</p>{app.data.tags?.length ? <div className="tag-list detail-tag-list">{app.data.tags.map((tag) => <span className="tag-badge" key={tag}>{tag}</span>)}</div> : null}</div><div className="detail-badges"><span className="environment-badge">{environmentLabel(app.data.environment)}</span><span className="app-type-badge">{applicationTypeLabel(app.data.appType, app.data.typeVersion)}</span><span className={`status-badge status-badge--${app.data.status === "active" ? "online" : "disabled"}`}>{app.data.status === "active" ? "启用" : "已归档"}</span></div></div>
    {isAdministrator ? <div className="detail-toolbar"><Button onClick={() => { const next = !editing; setEditing(next); if (next) { setEditingContract(false); discardContractDraft(); selectView("overview"); } else discardMetadataDraft(); }}>编辑应用</Button><Button tone={app.data.status === "active" ? "danger" : "default"} disabled={status.isPending} onClick={changeStatus}><Archive aria-hidden="true" />{app.data.status === "active" ? "归档应用" : "恢复应用"}</Button></div> : null}
    {status.error ? <ApiErrorNotice error={toNotice(status.error)} /> : null}
    <div className="detail-tabs" role="tablist" aria-label="应用详情视图">
      <button type="button" role="tab" aria-selected={view === "overview"} onClick={() => selectView("overview")}>概览</button>
      <button type="button" role="tab" aria-selected={view === "sources"} onClick={() => selectView("sources")}>部署来源</button>
      <button type="button" role="tab" aria-selected={view === "env"} onClick={() => selectView("env")}>运行配置</button>
      <button type="button" role="tab" aria-selected={view === "config"} onClick={() => selectView("config")}>配置副本</button>
      <button type="button" role="tab" aria-selected={view === "targets"} onClick={() => selectView("targets")}>部署目标</button>
    </div>
    <div role="tabpanel" aria-label="概览" hidden={view !== "overview"}>
      {editing ? <form className="node-form" onSubmit={(event) => void submit(event)}>
        <Field label="名称"><TextInput required value={name ?? app.data.name} onChange={(event) => setName(event.target.value)} /></Field>
        <Field label="Slug"><TextInput required value={slug ?? app.data.slug} onChange={(event) => setSlug(event.target.value)} /></Field>
        <Field label="环境"><Select required value={environment ?? app.data.environment} onChange={(event) => setEnvironment(event.target.value)}>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></Field>
        <Field label="应用类型"><Select value={`${appType ?? app.data.appType}/${typeVersion ?? app.data.typeVersion}`} onChange={(event) => { const [type, version] = event.target.value.split("/"); setAppType(type); setTypeVersion(version); }}>{APPLICATION_TYPE_OPTIONS.map((item) => <option key={`${item.type}/${item.version}`} value={`${item.type}/${item.version}`}>{item.label}</option>)}</Select></Field>
        <TagPickerField hint="一个应用可关联多个标签，用于区分项目或用途。" availableTags={availableTags} value={tags ?? app.data.tags ?? []} onChange={setTags} />
        <Field label="说明" className="form-span"><TextArea rows={3} value={description ?? app.data.description} onChange={(event) => setDescription(event.target.value)} /></Field>
        <div className="form-hint form-span">部署契约（参数 JSON Schema 与部署后验证配置）在「运行配置」中维护。</div>
        {update.error ? <div className="form-span"><ApiErrorNotice error={toNotice(update.error)} /></div> : null}
        <div className="form-actions form-span"><Button type="button" onClick={() => { setEditing(false); discardMetadataDraft(); }}>丢弃草稿</Button><Button tone="primary" disabled={update.isPending}>保存</Button></div>
      </form> : null}
      <section className="detail-section">
        <div className="section-heading"><div><h3>运行状态</h3><p>平台按「部署后验证配置」探测应用运行状态，最近一次结果如下。</p></div>{isAdministrator && app.data.status === "active" ? <div className="section-actions"><Button disabled={runtimeProbe.isPending || probing} onClick={() => runtimeProbe.mutate()}>{runtimeProbe.isPending || probing ? "正在检测..." : <><RefreshCw aria-hidden="true" />刷新运行状态</>}</Button></div> : null}</div>
        {runtimeProbe.error ? <ApiErrorNotice error={toNotice(runtimeProbe.error)} /> : null}
        <div className="definition-grid">
          <div><dt>当前状态</dt><dd><div className="runtime-summary">{applicationRuntimeBadge(app.data)}<span>{runtime.detail}</span></div></dd></div>
          <div><dt>最近部署</dt><dd>{app.data.lastDeployedAt ? formatTime(app.data.lastDeployedAt) : "尚未部署"}</dd></div>
          <div><dt>部署目标</dt><dd>{targets.items.length === 0 ? "尚未配置部署目标" : `${targets.items.length}${targets.hasNextPage ? "+" : ""} 个目标 · 启用 ${enabledTargets} 个`}</dd></div>
          <div><dt>最近检测</dt><dd>{app.data.runtimeCheckedAt ? formatTime(app.data.runtimeCheckedAt) : "尚无检测结果"}</dd></div>
        </div>
      </section>
      <section className="detail-section">
        <div className="section-heading"><div><h3>应用模块</h3><p>模块清单来自参数 JSON Schema 的 modules.x-options；部署时默认选中 x-default-selected 声明的模块。</p></div></div>
        {modules.length === 0 ? <p className="muted">参数 Schema 尚未声明应用模块，部署时按目标参数默认值执行。</p> : <ul className="module-chip-list">{modules.map((module) => <li className={`module-chip${defaultModules.includes(module) ? " module-chip--default" : ""}`} key={module}><span>{module}</span>{defaultModules.includes(module) ? <small>默认选中</small> : null}</li>)}</ul>}
      </section>
    </div>
    <div role="tabpanel" aria-label="部署来源" hidden={view !== "sources"}>
      <ApplicationSourceSection applicationId={id} isAdministrator={isAdministrator} applicationActive={app.data.status === "active"} />
      <WorkspaceSourceSection applicationId={id} isAdministrator={isAdministrator} applicationActive={app.data.status === "active"} />
    </div>
    <div role="tabpanel" aria-label="运行配置" hidden={view !== "env"}>
      <ApplicationEnvSection applicationId={id} isAdministrator={isAdministrator} />
      <section className="detail-section">
        <div className="section-heading"><div><h3>部署契约</h3><p>参数 Schema 与部署后验证配置按应用统一维护；部署目标读取并沿用应用级生效值。</p></div>{isAdministrator && !editingContract ? <div className="section-actions"><Button onClick={() => { setEditing(false); discardMetadataDraft(); setEditingContract(true); selectView("env"); }}>编辑契约</Button></div> : null}</div>
        {editingContract ? <form className="node-form" onSubmit={(event) => void submitContract(event)}>
          <Field label="参数 JSON Schema" hint="部署参数契约按应用统一配置；modules.x-options 声明可选模块，x-default-selected 可配置默认选中模块，省略时默认全选。" className="form-span"><TextArea rows={12} spellCheck={false} value={parameterSchema ?? JSON.stringify(app.data.parameterSchema ?? {}, null, 2)} onChange={(event) => setParameterSchema(event.target.value)} /></Field>
          <Field label="部署后验证配置" hint="部署完成后平台按此配置验证发布结果，按应用统一生效；概览的运行状态也按此配置探测。" className="form-span"><TextArea rows={12} spellCheck={false} value={verificationConfig ?? JSON.stringify(app.data.verificationConfig ?? {}, null, 2)} onChange={(event) => setVerificationConfig(event.target.value)} /></Field>
          {contractError ? <div className="notice notice--danger form-span" role="alert">{contractError}</div> : null}
          {contractUpdate.error ? <div className="form-span"><ApiErrorNotice error={toNotice(contractUpdate.error)} /></div> : null}
          <div className="form-actions form-span"><Button type="button" onClick={() => { setEditingContract(false); discardContractDraft(); }}>放弃修改</Button><Button tone="primary" disabled={contractUpdate.isPending}>{contractUpdate.isPending ? "正在保存..." : "保存契约"}</Button></div>
        </form> : <div className="contract-preview-grid">
          <div><h4>参数 JSON Schema</h4><pre className="json-preview">{JSON.stringify(app.data.parameterSchema ?? {}, null, 2)}</pre></div>
          <div><h4>部署后验证配置</h4><pre className="json-preview">{JSON.stringify(app.data.verificationConfig ?? {}, null, 2)}</pre></div>
        </div>}
      </section>
    </div>
    <div role="tabpanel" aria-label="配置副本" hidden={view !== "config"}>
      <ApplicationConfigSection applicationId={id} isAdministrator={isAdministrator} />
    </div>
    <div role="tabpanel" aria-label="部署目标" hidden={view !== "targets"}>
      <section className="detail-section"><div className="section-heading"><div><h3>部署目标</h3><p>应用部署会一次性固化并发布到全部启用目标；执行模式按目标配置，release 固定使用 Agent 原生特权发布。</p></div><div className="section-actions">{app.data.status === "active" && targets.items.some((target) => target.status === "active") ? <Link className="button button--primary" to={`/deployments/new?application=${id}`}><Play aria-hidden="true" />部署应用</Link> : null}{isAdministrator && app.data.status === "active" ? <Button onClick={() => setAddingTarget(true)}><Plus aria-hidden="true" />添加目标</Button> : null}</div></div>
        {addingTarget ? <TargetEditor applicationId={id} nodes={nodes.items} hasMoreNodes={nodes.hasNextPage} loadingMoreNodes={nodes.isFetchingNextPage} onLoadMoreNodes={() => void nodes.fetchNextPage()} onDiscard={() => setAddingTarget(false)} onSaved={() => setAddingTarget(false)} /> : targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : targets.items.length === 0 ? <PageState kind="empty" /> : <><div className="node-card-grid">{targets.items.map((target) => <TargetResourceCard key={target.id} applicationId={id} target={target} node={nodeById.get(target.nodeId)} isAdministrator={isAdministrator} />)}</div>{targets.hasNextPage ? <div className="pagination-actions"><Button onClick={() => void targets.fetchNextPage()}>加载更多</Button></div> : null}</>}
      </section>
    </div>
  </section>;
}

function TargetResourceCard({ applicationId, target, node, isAdministrator }: {
  applicationId: string;
  target: DeploymentTargetResponse;
  node?: NodeResponse;
  isAdministrator: boolean;
}) {
  const nodeOnline = node?.status === "online";
  const nodeStatusClass = !node ? "unknown" : nodeOnline ? "online" : "offline";
  const nodeStatusText = !node ? "节点未知" : nodeOnline ? "在线" : "离线";
  const telemetry = useQuery({
    queryKey: ["node", target.nodeId, "telemetry"],
    queryFn: ({ signal }) => applicationNodesApi.nodesTelemetry({ id: target.nodeId }, { signal }),
    enabled: nodeOnline,
    refetchInterval: nodeOnline ? 10_000 : false,
    refetchIntervalInBackground: false,
  });
  const latest = telemetry.data?.latest ?? null;
  const releasePath = target.imageSpec ? target.imageSpec.image : target.scriptPath;
  return <article className="node-card">
    <Link className="node-card__link" to={`/apps/${applicationId}/targets/${target.id}`} aria-label={`${isAdministrator ? "配置" : "查看"}目标 ${target.targetCode}`}>
      <div className="node-card__head">
        <span className="node-card__icon" aria-hidden="true"><Server /></span>
        <div className="node-card__identity"><h3>{node?.name ?? target.nodeId}</h3><p title={node?.host ?? target.nodeId}>{node?.host || "主机信息不可用"}</p></div>
        <span className={`node-card__status node-card__status--${nodeStatusClass}`}><span aria-hidden="true" />{nodeStatusText}</span>
      </div>
      <div className="node-card__meta">
        <span className={`status-badge status-badge--${target.status === "active" ? "online" : "disabled"}`}>{target.status === "active" ? "启用" : "停用"}</span>
        <span className="exec-mode-badge">{executionModeLabel(target.executionMode)}</span>
        <code className="target-code-badge">{target.targetCode}</code>
        {target.executionMode === "two_stage" || target.executionMode === "two_stage_script" || target.executionMode === "image" ? <span className="privilege-badge privilege-badge--enabled"><ShieldCheck aria-hidden="true" />{privilegedReleaseLabel()}</span> : null}
      </div>
      <div className="node-card__metrics" aria-label="节点资源">
        <NodeMetric label="CPU" value={latest?.cpuUsageRatio} format={formatPercent} />
        <NodeMetric label="内存" value={latest?.memoryUsedBytes} total={latest?.memoryTotalBytes} format={formatBytesPair} />
        <NodeMetric label="工作盘" value={latest?.workRootUsedBytes} total={latest?.workRootTotalBytes} format={formatBytesPair} />
      </div>
      <div className="target-card__spec"><span>发布物</span><code title={releasePath}>{releasePath}</code></div>
      <div className="node-card__foot"><span>目标 <code>{target.id}</code></span><span className="node-card__manage" aria-hidden="true">{isAdministrator ? "配置" : "查看"}</span></div>
    </Link>
  </article>;
}

function formatTime(value: string) {
  return new Date(value).toLocaleString("zh-CN");
}

function delay(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function parseJsonObject(value: string, label: string): Record<string, unknown> {
  let parsed: unknown;
  try { parsed = JSON.parse(value) as unknown; } catch { throw new Error(`${label} 不是有效 JSON`); }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new Error(`${label} 必须是 JSON object`);
  return parsed as Record<string, unknown>;
}
