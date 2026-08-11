import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import type { EnvEditorMode } from "../../api/contracts";
import { ApiError, normalizeApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { Field, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { applicationEnvsApi } from "./api";
import { RawEditor, ReauthenticationForm, StructuredEditor, ValidationErrors } from "./ApplicationEnvEditor";
import { assignments, parseDotenv, serializeDotenv } from "./dotenv";

interface GrantState { token: string; expiresAt: string }

const fileNamePattern = /^[A-Za-z0-9][A-Za-z0-9._-]*\.env$/;
const modulePattern = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

export function ApplicationEnvRegisterPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const metadata = useQuery({ queryKey: ["application-env-files", id], queryFn: () => applicationEnvsApi.applicationEnvsList({ applicationId: id }) });
  const existingNames = new Set((metadata.data?.items ?? []).map((item) => item.fileName.toLowerCase()));
  const [fileName, setFileName] = useState("");
  const [module, setModule] = useState("");
  const [draft, setDraft] = useState("");
  const [mode, setMode] = useState<EnvEditorMode>("structured");
  const [grant, setGrant] = useState<GrantState | null>(null);
  const [pending, setPending] = useState<"reauth" | "register" | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const document = useMemo(() => parseDotenv(draft), [draft]);
  const dirty = !submitted && (fileName !== "" || module !== "" || draft !== "");
  useUnsavedChanges(dirty);
  const normalizedName = fileName.trim().toLowerCase();
  const duplicateName = normalizedName !== "" && existingNames.has(normalizedName);
  const metaInvalid = !fileNamePattern.test(fileName.trim()) || !modulePattern.test(module.trim()) || duplicateName;
  const canSubmit = fileName.trim() !== "" && module.trim() !== "" && !metaInvalid && document.errors.length === 0;

  useEffect(() => {
    if (!grant) return;
    const remaining = Date.parse(grant.expiresAt) - Date.now();
    const timeout = window.setTimeout(() => setGrant(null), Math.min(Math.max(remaining, 0), 2_147_483_647));
    return () => window.clearTimeout(timeout);
  }, [grant]);

  useEffect(() => {
    if (!submitted) return;
    navigate(`/apps/${id}`, { replace: true });
  }, [id, navigate, submitted]);

  async function reauthenticate(password: string) {
    if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
    const response = await applicationEnvsApi.applicationEnvsReauthenticate({ applicationId: id, xCSRFToken: auth.csrfToken, envReauthenticateRequest: { password, action: "read_write" } });
    return { token: response.grantToken, expiresAt: response.expiresAt };
  }

  async function handleGrant(password: string) {
    setPending("reauth");
    setError(null);
    try {
      setGrant(await reauthenticate(password));
    } catch (cause) {
      setError(await normalizeApiError(cause));
    } finally {
      setPending(null);
    }
  }

  async function register() {
    if (!auth.csrfToken || !grant) return;
    setPending("register");
    setError(null);
    try {
      await applicationEnvsApi.applicationEnvsRegisterAdmin({
        applicationId: id,
        xEnvRevealGrant: grant.token,
        xCSRFToken: auth.csrfToken,
        registerAdminApplicationEnvsRequest: {
          files: [{ fileName: fileName.trim(), module: module.trim(), format: "dotenv-v1", content: draft }],
        },
      });
      setFileName("");
      setModule("");
      setDraft("");
      setGrant(null);
      setConfirm(false);
      setSubmitted(true);
      await queryClient.invalidateQueries({ queryKey: ["application-env-files", id] });
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      setError(apiError);
      if (apiError.status === 403) setGrant(null);
      setConfirm(false);
    } finally {
      setPending(null);
    }
  }

  if (metadata.isLoading) return <PageState kind="loading" />;
  if (metadata.isError) return <div className="state-with-action"><ApiErrorNotice error={metadata.error instanceof ApiError ? metadata.error : new ApiError(0, "unexpected_error", "无法读取 Env 元数据")} /><Button onClick={() => void metadata.refetch()}>重试</Button></div>;

  return <section className="workspace env-editor-page">
    <BackLink to={`/apps/${id}`} parentLabel="应用" />
    <div className="detail-title"><div><h2>登记运行配置</h2><p>首次登记的 Env 会加密保存，并自动同步到全部启用目标节点。</p></div></div>
    <div className="env-register-form">
      <div className="form-grid">
        <Field label="文件名"><TextInput required placeholder="例如 api.env" value={fileName} aria-invalid={fileName.trim() !== "" && (!fileNamePattern.test(fileName.trim()) || duplicateName)} onChange={(event) => setFileName(event.target.value)} /></Field>
        <Field label="模块"><TextInput required placeholder="例如 api" value={module} aria-invalid={module.trim() !== "" && !modulePattern.test(module.trim())} onChange={(event) => setModule(event.target.value)} /></Field>
        <Field label="格式"><TextInput readOnly value="dotenv-v1" /></Field>
      </div>
      {fileName.trim() !== "" && !fileNamePattern.test(fileName.trim()) ? <div className="notice notice--danger" role="alert"><strong>文件名必须以 .env 结尾，只允许字母、数字、点、下划线和连字符。</strong></div> : null}
      {duplicateName ? <div className="notice notice--danger" role="alert"><strong>已存在同名配置，请直接编辑已有文件。</strong></div> : null}
      {module.trim() !== "" && !modulePattern.test(module.trim()) ? <div className="notice notice--danger" role="alert"><strong>模块名只允许字母、数字、点、下划线和连字符。</strong></div> : null}
      <div className="env-editor-toolbar">
        <div className="segmented-control" aria-label="Env 编辑模式"><Button aria-pressed={mode === "structured"} onClick={() => setMode("structured")}>结构化模式</Button><Button aria-pressed={mode === "raw"} disabled={document.errors.length > 0 && mode === "raw"} onClick={() => setMode("raw")}>原文模式</Button></div>
        <div>{!grant ? null : <Button tone="primary" disabled={!canSubmit || pending === "register"} onClick={() => setConfirm(true)}>提交登记</Button>}</div>
      </div>
      {mode === "structured" ? <StructuredEditor document={document} onChange={(lines) => setDraft(serializeDotenv(lines))} /> : <RawEditor fileName={fileName || "new.env"} content={draft} errors={document.errors} onChange={setDraft} />}
      {mode === "structured" && document.errors.length > 0 ? <ValidationErrors errors={document.errors} /> : null}
      {error ? <ApiErrorNotice error={error} /> : null}
      {!grant ? <ReauthenticationForm title="重新验证管理员密码" submitLabel="验证并继续登记" pending={pending === "reauth"} error={error} onSubmit={(password) => void handleGrant(password)} /> : <p className="env-editor-footnote">明文授权有效至 {new Date(grant.expiresAt).toLocaleTimeString("zh-CN")}；离开页面后本页会清除明文与授权。</p>}
    </div>
    <ConfirmDialog open={confirm} title={`登记 ${fileName.trim()}？`} message={<><p>将登记 {fileName.trim()}（{module.trim()}）并自动同步到全部启用目标节点。</p><p>共 {assignments(document).length} 个变量；本次不会在任何页面或日志中显示明文值。</p></>} confirmLabel="确认登记" tone="primary" pending={pending === "register"} onClose={() => setConfirm(false)} onConfirm={() => void register()} />
  </section>;
}
