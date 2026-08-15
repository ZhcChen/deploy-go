import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Play, Plus, Server, ShieldCheck } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
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
import { applicationNodesApi, applicationsApi, deploymentTargetsApi } from "./api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { ApplicationSourceSection } from "./ApplicationSourceSection";
import { ApplicationEnvSection } from "../application-envs/ApplicationEnvSection";

const APPLICATION_TYPE_OPTIONS = [
  { type: "binary", version: "1", label: "普通二进制 v1" },
  { type: "redis", version: "7", label: "Redis v7" },
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
  const queryClient = useQueryClient();
  const isAdministrator = auth.user?.identity === "administrator";
  const [editing, setEditing] = useState(false);
  const [addingTarget, setAddingTarget] = useState(false);
  const app = useQuery({ queryKey: ["application", id], queryFn: () => applicationsApi.applicationsShow({ id }) });
  const targets = useCursorCollection(["deployment-targets", id], (after) => deploymentTargetsApi.deploymentTargetsList({ applicationId: id, limit: 20, after: after ?? undefined }));
  const nodes = useCursorCollection(["nodes", "target-options"], (after) => applicationNodesApi.nodesList({ limit: 200, after: after ?? undefined }));
  const nodeById = new Map(nodes.items.map((node) => [node.id, node]));
  const [name, setName] = useState<string | null>(null);
  const [slug, setSlug] = useState<string | null>(null);
  const [description, setDescription] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<string | null>(null);
  const [appType, setAppType] = useState<string | null>(null);
  const [typeVersion, setTypeVersion] = useState<string | null>(null);
  const [parameterSchema, setParameterSchema] = useState<string | null>(null);
  const [verificationConfig, setVerificationConfig] = useState<string | null>(null);
  const [contractError, setContractError] = useState<string | null>(null);
  useUnsavedChanges(editing && (name !== null || slug !== null || description !== null || environment !== null || appType !== null || typeVersion !== null || parameterSchema !== null || verificationConfig !== null));
  const update = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdate({ id, xCSRFToken: auth.csrfToken, saveApplicationRequest: { name: (name ?? app.data.name).trim(), slug: (slug ?? app.data.slug).trim(), description: (description ?? app.data.description).trim(), appType: appType ?? app.data.appType, typeVersion: typeVersion ?? app.data.typeVersion, environment: (environment ?? app.data.environment), parameterSchema: parseJsonObject(parameterSchema ?? JSON.stringify(app.data.parameterSchema ?? {}, null, 2), "参数 JSON Schema"), verificationConfig: parseJsonObject(verificationConfig ?? JSON.stringify(app.data.verificationConfig ?? {}, null, 2), "部署后验证配置"), version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); setEditing(false); } });
  const status = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdateStatus({ id, xCSRFToken: auth.csrfToken, applicationStatusRequest: { status: app.data.status === "active" ? "archived" : "active", version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); } });
  async function submit(event: FormEvent) {
    event.preventDefault();
    setContractError(null);
    try {
      parseJsonObject(parameterSchema ?? JSON.stringify(app.data?.parameterSchema ?? {}, null, 2), "参数 JSON Schema");
      parseJsonObject(verificationConfig ?? JSON.stringify(app.data?.verificationConfig ?? {}, null, 2), "部署后验证配置");
    } catch (error) {
      setContractError(error instanceof Error ? error.message : "部署契约 JSON 格式不正确");
      return;
    }
    await update.mutateAsync().catch(() => undefined);
  }
  function changeStatus() {
    if (app.data?.status === "active" && !window.confirm("归档后将阻止创建和执行新的部署目标，确定继续吗？")) return;
    status.mutate();
  }
  if (app.isLoading) return <PageState kind="loading" />;
  if (app.isError || !app.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(app.error)} /><Link className="button button--default" to="/apps">返回应用</Link></div>;
  return <section className="workspace detail-page">
    <BackLink to="/apps" parentLabel="应用列表" />
    <div className="detail-title"><div><h2>{app.data.name}</h2><p><code>{app.data.slug}</code> · {app.data.description || "暂无说明"}</p></div><div className="detail-badges"><span className="environment-badge">{environmentLabel(app.data.environment)}</span><span className="app-type-badge">{applicationTypeLabel(app.data.appType, app.data.typeVersion)}</span><span className={`status-badge status-badge--${app.data.status === "active" ? "online" : "disabled"}`}>{app.data.status === "active" ? "启用" : "已归档"}</span></div></div>
    {isAdministrator ? <div className="detail-toolbar"><Button onClick={() => setEditing((value) => !value)}>编辑应用</Button><Button tone={app.data.status === "active" ? "danger" : "default"} disabled={status.isPending} onClick={changeStatus}><Archive aria-hidden="true" />{app.data.status === "active" ? "归档应用" : "恢复应用"}</Button></div> : null}
    {status.error ? <ApiErrorNotice error={toNotice(status.error)} /> : null}
    {editing ? <form className="node-form" onSubmit={(event) => void submit(event)}>
      <Field label="名称"><TextInput required value={name ?? app.data.name} onChange={(event) => setName(event.target.value)} /></Field>
      <Field label="Slug"><TextInput required value={slug ?? app.data.slug} onChange={(event) => setSlug(event.target.value)} /></Field>
      <Field label="环境"><Select required value={environment ?? app.data.environment} onChange={(event) => setEnvironment(event.target.value)}>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></Field>
      <Field label="应用类型"><Select value={`${appType ?? app.data.appType}/${typeVersion ?? app.data.typeVersion}`} onChange={(event) => { const [type, version] = event.target.value.split("/"); setAppType(type); setTypeVersion(version); }}>{APPLICATION_TYPE_OPTIONS.map((item) => <option key={`${item.type}/${item.version}`} value={`${item.type}/${item.version}`}>{item.label}</option>)}</Select></Field>
      <Field label="说明" className="form-span"><TextArea rows={3} value={description ?? app.data.description} onChange={(event) => setDescription(event.target.value)} /></Field>
      <Field label="参数 JSON Schema" hint="部署参数契约按应用统一配置；同应用多个目标共用，不在目标上重复维护。" className="form-span"><TextArea rows={12} spellCheck={false} value={parameterSchema ?? JSON.stringify(app.data.parameterSchema ?? {}, null, 2)} onChange={(event) => setParameterSchema(event.target.value)} /></Field>
      <Field label="部署后验证配置" hint="部署完成后平台按此配置验证发布结果，按应用统一生效。" className="form-span"><TextArea rows={12} spellCheck={false} value={verificationConfig ?? JSON.stringify(app.data.verificationConfig ?? {}, null, 2)} onChange={(event) => setVerificationConfig(event.target.value)} /></Field>
      {contractError ? <div className="notice notice--danger form-span" role="alert">{contractError}</div> : null}
      {update.error ? <div className="form-span"><ApiErrorNotice error={toNotice(update.error)} /></div> : null}
      <div className="form-actions form-span"><Button type="button" onClick={() => { setEditing(false); setName(null); setSlug(null); setDescription(null); setEnvironment(null); setAppType(null); setTypeVersion(null); setParameterSchema(null); setVerificationConfig(null); setContractError(null); }}>丢弃草稿</Button><Button tone="primary" disabled={update.isPending}>保存</Button></div>
    </form> : null}
    <ApplicationSourceSection applicationId={id} isAdministrator={isAdministrator} applicationActive={app.data.status === "active"} />
    <ApplicationEnvSection applicationId={id} isAdministrator={isAdministrator} />
    <section className="detail-section">
      <div className="section-heading"><div><h3>部署契约</h3><p>参数 Schema 与部署后验证配置按应用统一维护；部署目标读取并沿用应用级生效值。</p></div></div>
      <div className="contract-preview-grid">
        <div><h4>参数 JSON Schema</h4><pre className="json-preview">{JSON.stringify(app.data.parameterSchema ?? {}, null, 2)}</pre></div>
        <div><h4>部署后验证配置</h4><pre className="json-preview">{JSON.stringify(app.data.verificationConfig ?? {}, null, 2)}</pre></div>
      </div>
    </section>
    <section className="detail-section"><div className="section-heading"><div><h3>部署目标</h3><p>应用部署会一次性固化并发布到全部启用目标；执行模式按目标配置，release 固定使用 Agent 原生特权发布。</p></div><div className="section-actions">{app.data.status === "active" && targets.items.some((target) => target.status === "active") ? <Link className="button button--primary" to={`/deployments/new?application=${id}`}><Play aria-hidden="true" />部署应用</Link> : null}{isAdministrator && app.data.status === "active" ? <Button onClick={() => setAddingTarget(true)}><Plus aria-hidden="true" />添加目标</Button> : null}</div></div>
      {addingTarget ? <TargetEditor applicationId={id} nodes={nodes.items} hasMoreNodes={nodes.hasNextPage} loadingMoreNodes={nodes.isFetchingNextPage} onLoadMoreNodes={() => void nodes.fetchNextPage()} onDiscard={() => setAddingTarget(false)} onSaved={() => setAddingTarget(false)} /> : targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : targets.items.length === 0 ? <PageState kind="empty" /> : <><ul className="resource-list target-list">{targets.items.map((target) => {
        const node = nodeById.get(target.nodeId);
        return <li key={target.id}><div className="target-list__identity"><Server aria-hidden="true" /><span><strong>{node?.name ?? target.nodeId}</strong><code>{target.nodeId}</code></span></div><div className="target-list__meta"><span className="exec-mode-badge">{executionModeLabel(target.executionMode)}</span><code className="target-code-badge">{target.targetCode}</code>{target.executionMode === "two_stage" || target.executionMode === "image" ? <span className="privilege-badge privilege-badge--enabled"><ShieldCheck aria-hidden="true" />{privilegedReleaseLabel()}</span> : null}<code className="target-list__path">{target.imageSpec ? target.imageSpec.image : target.scriptPath}</code></div><span className={`status-badge status-badge--${target.status === "active" ? "online" : "disabled"}`}>{target.status === "active" ? "启用" : "停用"}</span><span className="resource-actions"><Link className="text-link" to={`/apps/${id}/targets/${target.id}`}>{isAdministrator ? "配置" : "查看"}</Link></span></li>;
      })}</ul>{targets.hasNextPage ? <div className="pagination-actions"><Button onClick={() => void targets.fetchNextPage()}>加载更多</Button></div> : null}</>}
    </section>
  </section>;
}

function parseJsonObject(value: string, label: string): Record<string, unknown> {
  let parsed: unknown;
  try { parsed = JSON.parse(value) as unknown; } catch { throw new Error(`${label} 不是有效 JSON`); }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new Error(`${label} 必须是 JSON object`);
  return parsed as Record<string, unknown>;
}
