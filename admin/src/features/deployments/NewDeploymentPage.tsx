import { useMutation } from "@tanstack/react-query";
import { Play, Server } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field, Select } from "../../components/form";
import { PageState } from "../../components/PageState";
import { applicationsApi, deploymentTargetsApi } from "../applications/api";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { createIdempotencyKey, deploymentsApi } from "./api";
import { ModuleSelector, moduleOptions, ParameterEditor, schemaDefaults } from "./ParameterEditor";

export function NewDeploymentPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [search] = useSearchParams();
  const applications = useCursorCollection(["applications", "deployment-options"], (after) => applicationsApi.applicationsList({ limit: 100, after: after ?? undefined }));
  const [applicationId, setApplicationId] = useState(search.get("application") ?? "");
  const selectedApplicationId = applicationId || applications.items.find((item) => item.status === "active")?.id || "";
  const targets = useCursorCollection(["deployment-targets", selectedApplicationId, "deployment-options"], (after) => deploymentTargetsApi.deploymentTargetsList({ applicationId: selectedApplicationId, limit: 100, after: after ?? undefined }));
  const activeTargets = useMemo(() => targets.items.filter((target) => target.status === "active"), [targets.items]);
  const representativeTarget = activeTargets[0];
  const [parameterDrafts, setParameterDrafts] = useState<Record<string, Record<string, unknown>>>({});
  const parameters = representativeTarget ? parameterDrafts[selectedApplicationId] ?? schemaDefaults(representativeTarget.parameterSchema) : {};
  const isTwoStage = representativeTarget?.executionMode === "two_stage";
  const isImage = representativeTarget?.executionMode === "image";
  const configuredModules = representativeTarget ? moduleOptions(representativeTarget.parameterSchema) : [];
  const modulesValid = !isTwoStage || (configuredModules.length > 0 && String(parameters.modules ?? "").length > 0);
  const [dirty, setDirty] = useState(false);
  const [idempotencyKey, setIdempotencyKey] = useState("");
  const [releaseStrategy, setReleaseStrategy] = useState<"automatic" | "manual">("automatic");
  const confirmLock = useRef(false);
  const preview = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !selectedApplicationId) throw new Error("缺少必要的部署上下文");
      return deploymentsApi.preview(selectedApplicationId, auth.csrfToken, parameters, releaseStrategy);
    },
    onSuccess: () => { setIdempotencyKey(createIdempotencyKey("deploy")); setDirty(true); },
  });
  const confirm = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !preview.data || !idempotencyKey) throw new Error("请先重新预览部署");
      return deploymentsApi.confirm(selectedApplicationId, auth.csrfToken, idempotencyKey, preview.data.snapshotHash, parameters, releaseStrategy, preview.data.releaseVersion ?? undefined);
    },
    onSuccess: () => setDirty(false),
    onSettled: () => { confirmLock.current = false; },
  });
  const busy = preview.isPending || confirm.isPending;
  useUnsavedChanges(dirty && !confirm.isSuccess);
  const activeApplications = useMemo(() => applications.items.filter((item) => item.status === "active"), [applications.items]);

  useEffect(() => {
    if (confirm.data && !dirty) navigate(`/deployments/${confirm.data.id}`, { replace: true });
  }, [confirm.data, dirty, navigate]);

  function resetPreview() {
    preview.reset();
    setIdempotencyKey("");
  }

  function updateParameters(value: Record<string, unknown>) {
    setParameterDrafts((current) => ({ ...current, [selectedApplicationId]: value }));
    setDirty(true);
    resetPreview();
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    await preview.mutateAsync().catch(() => undefined);
  }

  function confirmDeployment() {
    if (confirmLock.current) return;
    confirmLock.current = true;
    confirm.mutate();
  }

  if (applications.isLoading) return <PageState kind="loading" />;
  if (applications.isError) return <ApiErrorNotice error={toNotice(applications.error)} />;

  return <section className="workspace deployment-create">
    <BackLink to="/deployments" parentLabel="部署列表" />
    <div className="workspace-heading"><div><h2>发起应用部署</h2><p>一次确认会固化全部启用目标，并分别记录每个节点的发布结果。</p></div></div>
    <form className="deployment-create-grid" onSubmit={(event) => void submit(event)}>
      <section className="deployment-step"><h3>1. 选择应用</h3>
        <Field label="应用"><Select disabled={busy} value={selectedApplicationId} onChange={(event) => { setApplicationId(event.target.value); setDirty(false); resetPreview(); }}><option value="">选择应用</option>{activeApplications.map((app) => <option key={app.id} value={app.id}>{app.name}</option>)}</Select></Field>
        {applications.hasNextPage ? <Button type="button" disabled={applications.isFetchingNextPage || busy} onClick={() => void applications.fetchNextPage()}>{applications.isFetchingNextPage ? "正在加载应用..." : "加载更多应用"}</Button> : null}
      </section>
      <section className="deployment-step" aria-labelledby="deployment-parameters-heading"><h3 id="deployment-parameters-heading">2. 配置部署</h3>
        {targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : activeTargets.length === 0 ? <p className="notice">该应用没有可部署目标</p> : isImage ? <p className="notice">镜像直连部署的镜像、模板、宿主端口与 Env 文件已由目标配置固定，无需在此配置参数。</p> : <><ParameterEditor schema={representativeTarget.parameterSchema} value={parameters} disabled={busy} hiddenNames={isTwoStage ? ["release-version", "modules"] : []} showEmpty={!isTwoStage} onChange={updateParameters} />{isTwoStage ? <ModuleSelector schema={representativeTarget.parameterSchema} value={parameters.modules} disabled={busy} onChange={(modules) => updateParameters({ ...parameters, modules })} /> : null}</>}
        {targets.hasNextPage ? <Button type="button" disabled={targets.isFetchingNextPage || busy} onClick={() => void targets.fetchNextPage()}>{targets.isFetchingNextPage ? "正在加载目标..." : "加载更多目标"}</Button> : null}
        {isTwoStage ? <Field label="发布方式"><div className="segmented-control" aria-label="发布方式"><Button type="button" aria-pressed={releaseStrategy === "automatic"} disabled={busy} onClick={() => { setReleaseStrategy("automatic"); setDirty(true); resetPreview(); }}>自动发布</Button><Button type="button" aria-pressed={releaseStrategy === "manual"} disabled={busy} onClick={() => { setReleaseStrategy("manual"); setDirty(true); resetPreview(); }}>构建后手动发布</Button></div></Field> : null}
        <div className="form-actions"><Button tone="primary" aria-label="生成部署预览" disabled={!selectedApplicationId || targets.isLoading || targets.isError || activeTargets.length === 0 || !modulesValid || busy}>{preview.isPending ? "正在生成预览..." : "生成部署预览"}</Button></div>
      </section>
    </form>
    {preview.error ? <ApiErrorNotice error={toNotice(preview.error)} /> : null}
    {preview.data ? <section className="deployment-preview" aria-label="部署预览">
      <div className="section-heading"><div><h3>3. 核对全部目标</h3><p>配置或目标变化会使当前 snapshot 失效。</p></div></div>
      <dl className="definition-grid"><div><dt>应用</dt><dd>{preview.data.applicationName}</dd></div><div><dt>目标数量</dt><dd>{preview.data.targets.length}</dd></div><div><dt>执行模式</dt><dd>{preview.data.executionMode === "two_stage" ? "两阶段（prepare + release）" : preview.data.executionMode === "image" ? "镜像直连（固定 Make target）" : "单脚本"}</dd></div>{preview.data.executionMode === "two_stage" ? <><div><dt>固定分支</dt><dd><code>{preview.data.deploymentBranch}</code></dd></div><div><dt>Commit</dt><dd><code>{preview.data.resolvedCommitSha}</code></dd></div><div><dt>发布版本</dt><dd><code>{preview.data.releaseVersion}</code></dd></div><div><dt>模块</dt><dd>{preview.data.modules?.join(", ")}</dd></div></> : preview.data.executionMode === "image" && preview.data.imageSpec ? <><div><dt>模板</dt><dd><code>{preview.data.imageSpec.template}</code></dd></div><div><dt>镜像</dt><dd><code>{preview.data.imageSpec.image}</code></dd></div><div><dt>宿主端口</dt><dd><code>{preview.data.imageSpec.host_port}</code></dd></div><div><dt>Env 文件</dt><dd>{preview.data.imageSpec.env_files.join(", ")}</dd></div></> : null}<div><dt>Snapshot</dt><dd><code>{preview.data.snapshotHash}</code></dd></div>{preview.data.executionMode === "image" ? null : <div><dt>参数</dt><dd><code>{JSON.stringify(preview.data.parameters)}</code></dd></div>}</dl>
      <ul className="deployment-target-preview" aria-label="目标节点预览">{preview.data.targets.map((target) => <li key={target.targetId}><div className="target-preview__identity"><Server aria-hidden="true" /><span><strong>{target.nodeName}</strong><code>{target.nodeId}</code></span></div><div className="target-preview__states"><span className={`status-badge status-badge--${target.agentOnline ? "online" : "pending"}`}>{target.agentOnline ? "在线" : "离线，部署将等待节点恢复"}</span><span className={`status-badge status-badge--${target.envGateStatus === "failed" ? "disabled" : target.envGateStatus === "ready" || target.envGateStatus === "not_required" ? "online" : "pending"}`}>{envGateLabel(target.envGateStatus)}</span></div><code>{target.imageSpec?.image ?? target.scriptPath}</code></li>)}</ul>
      {confirm.error ? <ApiErrorNotice error={toNotice(confirm.error)} /> : null}
      <div className="form-actions"><Button tone="primary" aria-label={`确认并发起部署，共 ${preview.data.targets.length} 个目标`} disabled={confirm.isPending} onClick={confirmDeployment}><Play aria-hidden="true" />{confirm.isPending ? "正在确认..." : "确认并发起部署"}</Button></div>
    </section> : null}
  </section>;
}

function envGateLabel(status: string) {
  if (status === "ready") return "Env 已就绪";
  if (status === "failed") return "Env 同步失败";
  if (status === "not_required") return "无需 Env";
  return "Env 等待同步";
}
