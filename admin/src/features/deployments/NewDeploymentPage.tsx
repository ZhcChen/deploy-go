import { useMutation } from "@tanstack/react-query";
import { ArrowLeft, Play } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { Field, Select } from "../../components/form";
import { PageState } from "../../components/PageState";
import { applicationsApi, deploymentTargetsApi } from "../applications/api";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { createIdempotencyKey, deploymentsApi } from "./api";
import { ParameterEditor, schemaDefaults } from "./ParameterEditor";

export function NewDeploymentPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [search] = useSearchParams();
  const applications = useCursorCollection(["applications", "deployment-options"], (after) => applicationsApi.applicationsList({ limit: 100, after: after ?? undefined }));
  const [applicationId, setApplicationId] = useState(search.get("application") ?? "");
  const selectedApplicationId = applicationId || applications.items.find((item) => item.status === "active")?.id || "";
  const targets = useCursorCollection(["deployment-targets", selectedApplicationId, "deployment-options"], (after) => deploymentTargetsApi.deploymentTargetsList({ applicationId: selectedApplicationId, limit: 100, after: after ?? undefined }));
  const [targetId, setTargetId] = useState(search.get("target") ?? "");
  const selectedTarget = targets.items.find((item) => item.id === targetId) ?? targets.items.find((item) => item.status === "active");
  const [parameterDrafts, setParameterDrafts] = useState<Record<string, Record<string, unknown>>>({});
  const [dirty, setDirty] = useState(false);
  const parameters = selectedTarget ? parameterDrafts[selectedTarget.id] ?? schemaDefaults(selectedTarget.parameterSchema) : {};
  const [idempotencyKey, setIdempotencyKey] = useState("");
  const confirmLock = useRef(false);
  const preview = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !selectedTarget) throw new Error("缺少必要的部署上下文");
    return deploymentsApi.preview(selectedTarget.id, auth.csrfToken, parameters);
  }, onSuccess: () => { setIdempotencyKey(createIdempotencyKey("deploy")); setDirty(true); } });
  const confirm = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !preview.data || !idempotencyKey) throw new Error("请先重新预览部署");
    return deploymentsApi.confirm(preview.data.targetId, auth.csrfToken, idempotencyKey, preview.data.snapshotHash, parameters);
  }, onSuccess: () => setDirty(false), onSettled: () => { confirmLock.current = false; } });
  const busy = preview.isPending || confirm.isPending;
  useUnsavedChanges(dirty && !confirm.isSuccess);
  const activeApplications = useMemo(() => applications.items.filter((item) => item.status === "active"), [applications.items]);
  useEffect(() => {
    if (confirm.data && !dirty) navigate(`/deployments/${confirm.data.id}`, { replace: true });
  }, [confirm.data, dirty, navigate]);
  function updateParameters(value: Record<string, unknown>) {
    if (selectedTarget) setParameterDrafts((current) => ({ ...current, [selectedTarget.id]: value }));
    setDirty(true);
    preview.reset();
    setIdempotencyKey("");
  }
  async function submit(event: FormEvent) { event.preventDefault(); await preview.mutateAsync().catch(() => undefined); }
  function confirmDeployment() {
    if (confirmLock.current) return;
    confirmLock.current = true;
    confirm.mutate();
  }
  if (applications.isLoading) return <PageState kind="loading" />;
  if (applications.isError) return <ApiErrorNotice error={toNotice(applications.error)} />;
  return <section className="workspace deployment-create"><Link className="back-link" to="/deployments"><ArrowLeft aria-hidden="true" />返回部署</Link><div className="workspace-heading"><div><h2>发起部署</h2><p>先生成服务端预览，再核对 snapshot 并确认执行。</p></div></div>
    <form className="deployment-create-grid" onSubmit={(event) => void submit(event)}>
      <section className="deployment-step"><h3>1. 选择目标</h3>
        <Field label="应用"><Select disabled={busy} value={selectedApplicationId} onChange={(event) => { setApplicationId(event.target.value); setTargetId(""); preview.reset(); setIdempotencyKey(""); }}><option value="">选择应用</option>{activeApplications.map((app) => <option key={app.id} value={app.id}>{app.name}</option>)}</Select></Field>
        {applications.hasNextPage ? <Button type="button" disabled={applications.isFetchingNextPage || busy} onClick={() => void applications.fetchNextPage()}>{applications.isFetchingNextPage ? "正在加载应用..." : "加载更多应用"}</Button> : null}
        <Field label="部署目标"><Select required disabled={busy || !selectedApplicationId} value={selectedTarget?.id ?? ""} onChange={(event) => { setTargetId(event.target.value); preview.reset(); setIdempotencyKey(""); }}><option value="">选择目标</option>{targets.items.filter((target) => target.status === "active").map((target) => <option key={target.id} value={target.id}>{target.environment} · {target.scriptPath}</option>)}</Select></Field>
        {targets.hasNextPage ? <Button type="button" disabled={targets.isFetchingNextPage || busy} onClick={() => void targets.fetchNextPage()}>{targets.isFetchingNextPage ? "正在加载目标..." : "加载更多目标"}</Button> : null}
      </section>
      <section className="deployment-step"><h3>2. 填写受控参数</h3>{targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : selectedTarget ? <ParameterEditor schema={selectedTarget.parameterSchema} value={parameters} disabled={busy} onChange={updateParameters} /> : <p className="notice">请先选择可用的部署目标。</p>}<div className="form-actions"><Button tone="primary" disabled={!selectedTarget || busy}>{preview.isPending ? "正在生成预览..." : "生成部署预览"}</Button></div></section>
    </form>
    {preview.error ? <ApiErrorNotice error={toNotice(preview.error)} /> : null}
    {preview.data ? <section className="deployment-preview" aria-label="部署预览"><div className="section-heading"><div><h3>3. 核对并确认</h3><p>配置变化会使当前 snapshot 失效。</p></div></div><dl className="definition-grid"><div><dt>应用</dt><dd>{preview.data.applicationName}</dd></div><div><dt>节点</dt><dd>{preview.data.nodeName}</dd></div><div><dt>环境</dt><dd>{preview.data.environment}</dd></div><div><dt>脚本</dt><dd><code>{preview.data.scriptPath}</code></dd></div><div><dt>Snapshot</dt><dd><code>{preview.data.snapshotHash}</code></dd></div><div><dt>参数</dt><dd><code>{JSON.stringify(preview.data.parameters)}</code></dd></div></dl>{confirm.error ? <ApiErrorNotice error={toNotice(confirm.error)} /> : null}<div className="form-actions"><Button tone="primary" disabled={confirm.isPending} onClick={confirmDeployment}><Play aria-hidden="true" />{confirm.isPending ? "正在确认..." : "确认并发起部署"}</Button></div></section> : null}
  </section>;
}
