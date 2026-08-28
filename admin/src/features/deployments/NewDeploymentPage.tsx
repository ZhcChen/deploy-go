import { useMutation } from "@tanstack/react-query";
import { Check, Play, Server } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field } from "../../components/form";
import { PageState } from "../../components/PageState";
import { applicationsApi, deploymentTargetsApi } from "../applications/api";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { createIdempotencyKey, deploymentsApi } from "./api";
import { ModuleSelector, moduleOptions, ParameterEditor, schemaDefaults } from "./ParameterEditor";
import { ApplicationConfigWorkspace } from "../application-configs/ApplicationConfigWorkspace";

const LAST_APPLICATION_STORAGE_KEY = "deploy-go.deployments.last-application";

function initialApplicationId(urlValue: string) {
  if (urlValue) return urlValue;
  try {
    return window.localStorage.getItem(LAST_APPLICATION_STORAGE_KEY) ?? "";
  } catch {
    // 严格的浏览器策略可能禁用 localStorage。
    return "";
  }
}

function rememberApplication(applicationId: string) {
  if (!applicationId) return;
  try {
    window.localStorage.setItem(LAST_APPLICATION_STORAGE_KEY, applicationId);
  } catch {
    // 无法持久化时仍保留当前页面内的选择。
  }
}

export function NewDeploymentPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [search] = useSearchParams();
  const applications = useCursorCollection(["applications", "deployment-options"], (after) => applicationsApi.applicationsList({ limit: 200, after: after ?? undefined }));
  const { fetchNextPage: fetchNextApplicationsPage, hasNextPage: hasNextApplicationsPage, isFetchingNextPage: isFetchingNextApplicationsPage, isError: hasApplicationsLoadError } = applications;
  const [applicationId, setApplicationId] = useState(() => initialApplicationId(search.get("application") ?? ""));
  const activeApplications = useMemo(() => applications.items.filter((item) => item.status === "active"), [applications.items]);
  const sortedActiveApplications = useMemo(() => activeApplications.slice().sort(compareApplicationsByRecentDeployment), [activeApplications]);
  const selectedApplicationId = useMemo(() => {
    if (applicationId && sortedActiveApplications.some((item) => item.id === applicationId)) return applicationId;
    return sortedActiveApplications[0]?.id ?? "";
  }, [applicationId, sortedActiveApplications]);
  const targets = useCursorCollection(["deployment-targets", selectedApplicationId, "deployment-options"], (after) => selectedApplicationId ? deploymentTargetsApi.deploymentTargetsList({ applicationId: selectedApplicationId, limit: 100, after: after ?? undefined }) : Promise.resolve({ items: [], nextCursor: null }));
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
    onSuccess: () => {
      setDirty(false);
      rememberApplication(selectedApplicationId);
    },
    onSettled: () => { confirmLock.current = false; },
  });
  const busy = preview.isPending || confirm.isPending;
  useUnsavedChanges(dirty && !confirm.isSuccess);

  useEffect(() => {
    if (hasNextApplicationsPage && !isFetchingNextApplicationsPage && !hasApplicationsLoadError) void fetchNextApplicationsPage();
  }, [fetchNextApplicationsPage, hasApplicationsLoadError, hasNextApplicationsPage, isFetchingNextApplicationsPage]);

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
    <div className="deployment-create__body">
      <form className="deployment-create-grid" onSubmit={(event) => void submit(event)}>
      <section className="deployment-step deployment-step--applications" aria-labelledby="deployment-application-heading">
        <h3 id="deployment-application-heading">1. 选择应用</h3>
        <div className="deployment-application-list" aria-label="选择应用" role="listbox" aria-busy={isFetchingNextApplicationsPage || undefined}>
          {sortedActiveApplications.length === 0 ? <p className="deployment-application-empty">暂无启用应用</p> : sortedActiveApplications.map((app) => {
            const selected = app.id === selectedApplicationId;
            return <button className="deployment-application-item" key={app.id} type="button" role="option" aria-selected={selected} disabled={busy} onClick={() => { setApplicationId(app.id); rememberApplication(app.id); setDirty(false); resetPreview(); }}>
              <span className="deployment-application-item__check">{selected ? <Check aria-hidden="true" /> : null}</span>
              <span className="deployment-application-item__content"><strong>{app.name}</strong><small>{app.slug} · {app.lastDeployedAt ? `最近部署 ${formatApplicationDeploymentTime(app.lastDeployedAt)}` : "尚未部署"}</small></span>
            </button>;
          })}
          {isFetchingNextApplicationsPage ? <p className="deployment-application-loading">正在加载应用...</p> : null}
        </div>
      </section>
      <section className="deployment-step deployment-step--configuration" aria-labelledby="deployment-parameters-heading"><h3 id="deployment-parameters-heading">2. 配置部署</h3>
        {targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : activeTargets.length === 0 ? <p className="notice">该应用没有可部署目标</p> : isImage ? <><p className="notice">镜像与宿主端口由目标配置固定；模板配置已克隆为应用配置副本，保存后需重新生成预览。</p><ApplicationConfigWorkspace applicationId={selectedApplicationId} embedded height="min(46vh, 520px)" onSaved={() => { setDirty(true); resetPreview(); }} /></> : <><ParameterEditor schema={representativeTarget.parameterSchema} value={parameters} disabled={busy} hiddenNames={isTwoStage ? ["release-version", "modules"] : []} showEmpty={!isTwoStage} onChange={updateParameters} />{isTwoStage ? <ModuleSelector schema={representativeTarget.parameterSchema} value={parameters.modules} disabled={busy} onChange={(modules) => updateParameters({ ...parameters, modules })} /> : null}</>}
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
    </div>
  </section>;
}

function envGateLabel(status: string) {
  if (status === "ready") return "Env 已就绪";
  if (status === "failed") return "Env 同步失败";
  if (status === "not_required") return "无需 Env";
  return "Env 等待同步";
}

function compareApplicationsByRecentDeployment(left: { lastDeployedAt?: string | null; createdAt: string; id: string }, right: { lastDeployedAt?: string | null; createdAt: string; id: string }) {
  const difference = applicationDeploymentTime(right.lastDeployedAt) - applicationDeploymentTime(left.lastDeployedAt);
  if (difference !== 0) return difference;
  const createdAtDifference = right.createdAt.localeCompare(left.createdAt);
  return createdAtDifference !== 0 ? createdAtDifference : right.id.localeCompare(left.id);
}

function applicationDeploymentTime(value?: string | null) {
  if (!value) return Number.NEGATIVE_INFINITY;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : Number.NEGATIVE_INFINITY;
}

function formatApplicationDeploymentTime(value: string) {
  try {
    return new Date(value).toLocaleString("zh-CN");
  } catch {
    return value;
  }
}
