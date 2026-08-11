import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import type { ApplicationEnvPlaintextResponse } from "../../api/generated/models/ApplicationEnvPlaintextResponse";
import type { EnvGrantAction } from "../../api/generated/models/EnvGrantAction";
import { ApiError, normalizeApiError } from "../../api/http-client";
import type { EnvEditorMode } from "../../api/contracts";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { applicationEnvsApi } from "./api";
import { maskedDiff, parseDotenv, serializeDotenv } from "./dotenv";
import { RawEditor, ReauthenticationForm, StructuredEditor, ValidationErrors } from "./ApplicationEnvEditor";

interface GrantState { token: string; expiresAt: string }

export function ApplicationEnvEditorPage() {
  const { id = "", envFileId = "" } = useParams();
  const auth = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const metadata = useQuery({ queryKey: ["application-env-files", id], queryFn: () => applicationEnvsApi.applicationEnvsList({ applicationId: id }) });
  const file = metadata.data?.items.find((item) => item.id === envFileId);
  const [grant, setGrant] = useState<GrantState | null>(null);
  const [plaintext, setPlaintext] = useState<ApplicationEnvPlaintextResponse | null>(null);
  const [original, setOriginal] = useState("");
  const [draft, setDraft] = useState("");
  const [mode, setMode] = useState<EnvEditorMode>("structured");
  const [pending, setPending] = useState<"reauth" | "reveal" | "save" | "delete-auth" | "delete" | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [conflict, setConflict] = useState(false);
  const [saveConfirm, setSaveConfirm] = useState(false);
  const [deleteAuth, setDeleteAuth] = useState(false);
  const [deleteGrant, setDeleteGrant] = useState<GrantState | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const document = useMemo(() => parseDotenv(draft), [draft]);
  const dirty = plaintext !== null && draft !== original;
  const diff = useMemo(() => maskedDiff(original, draft), [draft, original]);
  useUnsavedChanges(dirty);

  useEffect(() => {
    if (!grant) return;
    const remaining = Date.parse(grant.expiresAt) - Date.now();
    if (remaining <= 0) {
      clearPlaintext();
      return;
    }
    const timeout = window.setTimeout(clearPlaintext, Math.min(remaining, 2_147_483_647));
    return () => window.clearTimeout(timeout);
  }, [grant]);

  function clearPlaintext() {
    setGrant(null);
    setDeleteGrant(null);
    setPlaintext(null);
    setOriginal("");
    setDraft("");
    setSaveConfirm(false);
    setDeleteAuth(false);
    setDeleteConfirm(false);
  }

  async function reauthenticate(password: string, action: EnvGrantAction) {
    if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
    const response = await applicationEnvsApi.applicationEnvsReauthenticate({ applicationId: id, xCSRFToken: auth.csrfToken, envReauthenticateRequest: { password, action } });
    return { token: response.grantToken, expiresAt: response.expiresAt };
  }

  async function reveal(nextGrant: GrantState) {
    if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
    setPending("reveal");
    const response = await applicationEnvsApi.applicationEnvsReveal({ envFileId, xEnvRevealGrant: nextGrant.token, xCSRFToken: auth.csrfToken });
    setPlaintext(response);
    setOriginal(response.content);
    setDraft(response.content);
    setConflict(false);
    setMode("structured");
  }

  async function handleReadGrant(password: string) {
    setPending("reauth");
    setError(null);
    try {
      const nextGrant = await reauthenticate(password, "read_write");
      setGrant(nextGrant);
      await reveal(nextGrant);
    } catch (cause) {
      clearPlaintext();
      setError(await normalizeApiError(cause));
    } finally {
      setPending(null);
    }
  }

  async function save() {
    if (!auth.csrfToken || !plaintext || !grant) return;
    setPending("save");
    setError(null);
    try {
      const saved = await applicationEnvsApi.applicationEnvsUpdate({ envFileId, xEnvRevealGrant: grant.token, xCSRFToken: auth.csrfToken, updateApplicationEnvRequest: { content: draft, expectedVersion: plaintext.version } });
      setPlaintext(saved);
      setOriginal(saved.content);
      setDraft(saved.content);
      setConflict(false);
      setSaveConfirm(false);
      await queryClient.invalidateQueries({ queryKey: ["application-env-files", id] });
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      setError(apiError);
      if (apiError.status === 403) clearPlaintext();
      setConflict(apiError.status === 409 || apiError.code === "version_conflict");
      setSaveConfirm(false);
    } finally {
      setPending(null);
    }
  }

  async function reloadLatest() {
    if (!grant) return;
    setError(null);
    try {
      await reveal(grant);
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      setConflict(false);
      if (apiError.status === 403) clearPlaintext();
      setError(apiError);
    } finally {
      setPending(null);
    }
  }

  async function handleDeleteGrant(password: string) {
    setPending("delete-auth");
    setError(null);
    try {
      const nextGrant = await reauthenticate(password, "delete");
      setDeleteGrant(nextGrant);
      setDeleteAuth(false);
      setDeleteConfirm(true);
    } catch (cause) {
      setError(await normalizeApiError(cause));
    } finally {
      setPending(null);
    }
  }

  async function remove() {
    if (!auth.csrfToken || !plaintext || !deleteGrant) return;
    setPending("delete");
    setError(null);
    try {
      await applicationEnvsApi.applicationEnvsDelete({ envFileId, xEnvRevealGrant: deleteGrant.token, xCSRFToken: auth.csrfToken, deleteApplicationEnvRequest: { expectedVersion: plaintext.version, confirmFileName: plaintext.fileName } });
      clearPlaintext();
      await queryClient.invalidateQueries({ queryKey: ["application-env-files", id] });
      navigate(`/apps/${id}`, { replace: true });
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      setError(apiError);
      setConflict(apiError.status === 409 || apiError.code === "version_conflict");
      setDeleteConfirm(false);
    } finally {
      setPending(null);
    }
  }

  if (metadata.isLoading) return <PageState kind="loading" />;
  if (metadata.isError) return <div className="state-with-action"><ApiErrorNotice error={metadata.error instanceof ApiError ? metadata.error : new ApiError(0, "unexpected_error", "无法读取 Env 元数据")} /><Button onClick={() => void metadata.refetch()}>重试</Button></div>;
  if (!file) return <section className="workspace"><h2>配置文件不存在</h2><Link className="button button--default" to={`/apps/${id}`}>返回应用</Link></section>;

  return <section className="workspace env-editor-page">
    <BackLink to={`/apps/${id}`} parentLabel="应用" />
    <div className="detail-title"><div><h2>{file.fileName}</h2><p>{file.module} · {file.format} · 当前版本 v{file.currentVersion}</p></div><div className="env-sync-summary"><span className="sync-state sync-state--pending">待同步 {file.pendingCount}</span><span className="sync-state sync-state--syncing">同步中 {file.syncingCount}</span><span className="sync-state sync-state--succeeded">已同步 {file.succeededCount}</span><span className="sync-state sync-state--failed">失败 {file.failedCount}</span></div></div>
    {!plaintext ? <ReauthenticationForm title="重新验证管理员密码" submitLabel="验证并读取" pending={pending === "reauth" || pending === "reveal"} error={error} onSubmit={(password) => void handleReadGrant(password)} /> : <>
      <div className="env-editor-toolbar">
        <div className="segmented-control" aria-label="Env 编辑模式"><Button aria-pressed={mode === "structured"} onClick={() => setMode("structured")}>结构化模式</Button><Button aria-pressed={mode === "raw"} disabled={document.errors.length > 0 && mode === "raw"} onClick={() => setMode("raw")}>原文模式</Button></div>
        <div><Button tone="danger" onClick={() => setDeleteAuth(true)}><Trash2 aria-hidden="true" />删除 Env</Button><Button tone="primary" disabled={!dirty || document.errors.length > 0 || pending === "save"} onClick={() => setSaveConfirm(true)}>保存 Env</Button></div>
      </div>
      {conflict ? <div className="notice notice--warning" role="alert"><strong>配置已被其他管理员更新，当前草稿不会覆盖最新版本。</strong><Button onClick={() => void reloadLatest()}>重新加载最新版本</Button></div> : error ? <ApiErrorNotice error={error} /> : null}
      {mode === "structured" ? <StructuredEditor document={document} onChange={(lines) => setDraft(serializeDotenv(lines))} /> : <RawEditor fileName={file.fileName} content={draft} errors={document.errors} onChange={setDraft} />}
      {mode === "structured" && document.errors.length > 0 ? <ValidationErrors errors={document.errors} /> : null}
      <p className="env-editor-footnote">明文授权有效至 {new Date(grant?.expiresAt ?? "").toLocaleTimeString("zh-CN")}；离开页面后本页会清除明文与授权。</p>
    </>}
    {deleteAuth ? <ReauthenticationForm title="验证后删除 Env" submitLabel="验证并继续删除" pending={pending === "delete-auth"} error={error} onCancel={() => setDeleteAuth(false)} onSubmit={(password) => void handleDeleteGrant(password)} /> : null}
    <ConfirmDialog open={saveConfirm} title={`保存 ${file.fileName}？`} message={<><p>保存后将立即同步到 {file.targetCount} 个目标节点。</p><pre className="env-masked-diff">{diff.map((line) => <span key={line}>{line}</span>)}</pre></>} confirmLabel="确认保存" tone="primary" pending={pending === "save"} onClose={() => setSaveConfirm(false)} onConfirm={() => void save()} />
    <ConfirmDialog open={deleteConfirm} title={`删除 ${file.fileName}？`} message={<p>该操作影响 {file.targetCount} 个目标节点，节点上的对应文件将被删除。业务应用后续重新登记前无法在 Web 恢复。</p>} confirmLabel="确认删除" pending={pending === "delete"} onClose={() => setDeleteConfirm(false)} onConfirm={() => void remove()} />
  </section>;
}
