import { useQuery, useQueryClient } from "@tanstack/react-query";
import { FileCode2, History, Lock, Save, ShieldAlert } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import type { ApplicationConfigFileResponse } from "../../api/generated/models/ApplicationConfigFileResponse";
import type { ApplicationConfigVersionResponse } from "../../api/generated/models/ApplicationConfigVersionResponse";
import type { ConfigDiagnostic } from "../../api/generated/models/ConfigDiagnostic";
import { ApiError, normalizeApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { CodeEditor } from "../../components/CodeEditor";
import { Field, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { applicationConfigsApi } from "./api";

interface GrantState {
  token: string;
  expiresAt: string;
}

interface BufferState {
  content: string;
  original: string;
  version: number;
}

interface WorkspaceProps {
  applicationId: string;
  embedded?: boolean;
  height?: string;
  onSaved?: () => void;
}

const formatLabels: Record<string, string> = {
  yaml: "YAML",
  dotenv: "Env",
  ini: "INI",
  json: "JSON",
  markdown: "Markdown",
  shell: "Shell",
  makefile: "Makefile",
};

function formatLabel(format: string) {
  return formatLabels[format] ?? format;
}

function sourceLabel(source: string) {
  switch (source) {
    case "template":
      return "模板克隆";
    case "legacy_initialization":
      return "历史 Env 初始化";
    case "restore_version":
      return "恢复历史版本";
    case "restore_template":
      return "恢复模板默认";
    default:
      return "用户保存";
  }
}

function ReauthPanel({
  pending,
  error,
  onCancel,
  onSubmit,
}: {
  pending: boolean;
  error: ApiError | null;
  onCancel(): void;
  onSubmit(password: string): void;
}) {
  const [password, setPassword] = useState("");
  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(password);
    setPassword("");
  }
  return (
    <section className="env-reauth config-reauth">
      <h3>重新验证管理员密码</h3>
      <p>敏感配置读取和保存会签发 5 分钟临时授权；授权不会延长当前登录会话。</p>
      <form onSubmit={submit}>
        <Field label="管理员密码">
          <TextInput
            required
            autoComplete="current-password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </Field>
        {error ? <ApiErrorNotice error={error} /> : null}
        <div className="form-actions">
          <Button type="button" disabled={pending} onClick={onCancel}>
            取消
          </Button>
          <Button tone="primary" disabled={pending || !password}>
            {pending ? "正在验证..." : "验证并继续"}
          </Button>
        </div>
      </form>
    </section>
  );
}

export function ApplicationConfigWorkspace({ applicationId, embedded = false, height, onSaved }: WorkspaceProps) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const files = useQuery({
    queryKey: ["application-config-files", applicationId],
    queryFn: () => applicationConfigsApi.applicationConfigsList({ applicationId }),
  });
  const items = useMemo(() => files.data?.items ?? [], [files.data?.items]);
  const editable = useMemo(() => items.filter((file) => file.editable), [items]);
  const [selectedId, setSelectedId] = useState("");
  const selected = editable.find((file) => file.id === selectedId) ?? editable[0] ?? items.find((file) => file.id === selectedId);
  const [buffers, setBuffers] = useState<Record<string, BufferState>>({});
  const [grant, setGrant] = useState<GrantState | null>(null);
  const [reauthOpen, setReauthOpen] = useState(false);
  const [pending, setPending] = useState<"reauth" | "reveal" | "save" | "validate" | "restore" | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [diagnostics, setDiagnostics] = useState<ConfigDiagnostic[]>([]);
  const [versionsOpen, setVersionsOpen] = useState(false);
  const [versions, setVersions] = useState<ApplicationConfigVersionResponse[]>([]);
  const dirty = selected ? Boolean(buffers[selected.id] && buffers[selected.id].content !== buffers[selected.id].original) : false;
  const buffer = selected ? buffers[selected.id] : null;

  useUnsavedChanges(editable.some((file) => buffers[file.id] && buffers[file.id].content !== buffers[file.id].original));

  const clearGrant = useCallback(() => {
    setGrant(null);
    setBuffers((current) => {
      const next = { ...current };
      for (const file of items) {
        if (file.sensitive) delete next[file.id];
      }
      return next;
    });
    setReauthOpen(false);
  }, [items]);

  useEffect(() => {
    if (!grant) return;
    const remaining = Math.max(0, Date.parse(grant.expiresAt) - Date.now());
    const timeout = window.setTimeout(clearGrant, Math.min(remaining, 2_147_483_647));
    return () => window.clearTimeout(timeout);
  }, [clearGrant, grant]);

  async function reauthenticate(password: string) {
    if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
    const response = await applicationConfigsApi.applicationConfigsReauthenticate({
      applicationId,
      xCSRFToken: auth.csrfToken,
      configReauthenticateRequest: { password, action: "read_write" },
    });
    return { token: response.grantToken, expiresAt: response.expiresAt } satisfies GrantState;
  }

  const reveal = useCallback(async (file: ApplicationConfigFileResponse, nextGrant?: GrantState) => {
    if (!auth.csrfToken || !file) return;
    if (file.sensitive && !nextGrant && !grant) {
      setReauthOpen(true);
      return;
    }
    setPending("reveal");
    setError(null);
    setDiagnostics([]);
    try {
      const response = await applicationConfigsApi.applicationConfigsShow({
        id: file.id,
        xCSRFToken: auth.csrfToken,
        xEnvRevealGrant: file.sensitive ? (nextGrant?.token ?? grant?.token) : null,
      });
      setBuffers((current) => ({
        ...current,
        [file.id]: { content: response.content ?? "", original: response.content ?? "", version: response.version },
      }));
      setSelectedId(file.id);
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      if (apiError.status === 403) clearGrant();
      setError(apiError);
    } finally {
      setPending(null);
    }
  }, [auth.csrfToken, clearGrant, grant]);

  const revealQuietly = useCallback(async (file: ApplicationConfigFileResponse) => {
    if (!auth.csrfToken || file.sensitive) return;
    try {
      const response = await applicationConfigsApi.applicationConfigsShow({
        id: file.id,
        xCSRFToken: auth.csrfToken,
        xEnvRevealGrant: null,
      });
      setBuffers((current) => ({
        ...current,
        [file.id]: { content: response.content ?? "", original: response.content ?? "", version: response.version },
      }));
      setSelectedId(file.id);
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      if (apiError.status === 403) clearGrant();
      setError(apiError);
    }
  }, [auth.csrfToken, clearGrant]);

  const autoRevealedApplication = useRef<string | null>(null);
  useEffect(() => {
    if (files.isLoading || files.isError || autoRevealedApplication.current === applicationId) return;
    const firstVisible = editable.find((file) => !file.sensitive);
    if (!firstVisible) return;
    autoRevealedApplication.current = applicationId;
    const timer = window.setTimeout(() => void revealQuietly(firstVisible), 0);
    return () => window.clearTimeout(timer);
  }, [applicationId, editable, files.isError, files.isLoading, revealQuietly]);

  async function handleReauth(password: string) {
    if (!selected) return;
    setPending("reauth");
    setError(null);
    try {
      const nextGrant = await reauthenticate(password);
      setGrant(nextGrant);
      setReauthOpen(false);
      await reveal(selected, nextGrant);
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      setError(apiError);
    } finally {
      setPending(null);
    }
  }

  async function save() {
    if (!auth.csrfToken || !selected || !buffer) return;
    if (selected.sensitive && !grant) {
      setReauthOpen(true);
      return;
    }
    setPending("save");
    setError(null);
    try {
      const saved = await applicationConfigsApi.applicationConfigsUpdate({
        id: selected.id,
        xCSRFToken: auth.csrfToken,
        xEnvRevealGrant: selected.sensitive ? grant?.token : null,
        updateApplicationConfigRequest: {
          content: buffer.content,
          expectedVersion: buffer.version,
        },
      });
      setBuffers((current) => ({
        ...current,
        [selected.id]: {
          content: saved.content ?? buffer.content,
          original: saved.content ?? buffer.content,
          version: saved.version,
        },
      }));
      setDiagnostics([]);
      await queryClient.invalidateQueries({ queryKey: ["application-config-files", applicationId] });
      await queryClient.invalidateQueries({ queryKey: ["application-env-files", applicationId] });
      onSaved?.();
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      if (apiError.status === 403) clearGrant();
      setError(apiError);
    } finally {
      setPending(null);
    }
  }

  async function validate() {
    if (!auth.csrfToken || !selected) return;
    if (selected.sensitive && !grant) {
      setReauthOpen(true);
      return;
    }
    setPending("validate");
    setError(null);
    try {
      const response = await applicationConfigsApi.applicationConfigsValidate({
        id: selected.id,
        xCSRFToken: auth.csrfToken,
        xEnvRevealGrant: selected.sensitive ? grant?.token : null,
        validateApplicationConfigRequest: { content: buffer?.content },
      });
      setDiagnostics(response.diagnostics);
    } catch (cause) {
      setError(await normalizeApiError(cause));
    } finally {
      setPending(null);
    }
  }

  async function openVersions() {
    if (!selected) return;
    setVersionsOpen(true);
    setError(null);
    try {
      const response = await applicationConfigsApi.applicationConfigsVersions({ id: selected.id });
      setVersions(response.items);
    } catch (cause) {
      setError(await normalizeApiError(cause));
    }
  }

  async function restoreVersion(version: number) {
    if (!auth.csrfToken || !selected || !buffer) return;
    if (selected.sensitive && !grant) {
      setReauthOpen(true);
      return;
    }
    setPending("restore");
    setError(null);
    try {
      const saved = await applicationConfigsApi.applicationConfigsRestore({
        id: selected.id,
        xCSRFToken: auth.csrfToken,
        xEnvRevealGrant: selected.sensitive ? grant?.token : null,
        restoreApplicationConfigRequest: {
          version,
          expectedVersion: buffer.version,
        },
      });
      setBuffers((current) => ({
        ...current,
        [selected.id]: {
          content: saved.content ?? "",
          original: saved.content ?? "",
          version: saved.version,
        },
      }));
      setVersionsOpen(false);
      await queryClient.invalidateQueries({ queryKey: ["application-config-files", applicationId] });
      await queryClient.invalidateQueries({ queryKey: ["application-env-files", applicationId] });
      onSaved?.();
    } catch (cause) {
      const apiError = await normalizeApiError(cause);
      if (apiError.status === 403) clearGrant();
      setError(apiError);
    } finally {
      setPending(null);
    }
  }

  if (files.isLoading) return <PageState kind="loading" />;
  if (files.isError) return <ApiErrorNotice error={files.error} />;
  if (items.length === 0) {
    return (
      <div className={`config-workspace ${embedded ? "config-workspace--embedded" : ""}`}>
        <div className="empty-inline">
          <p>该应用还没有可编辑的模板配置副本。请从应用模板创建应用，或在镜像目标上初始化配置工作区。</p>
        </div>
      </div>
    );
  }

  return (
    <section className={`config-workspace ${embedded ? "config-workspace--embedded" : ""}`} aria-label="应用配置工作区">
      {!embedded ? (
        <div className="workspace-heading">
          <div>
            <h2>应用配置</h2>
            <p>编辑模板克隆出的应用配置副本；保存后生成新版本，部署 preview 会固化当前版本摘要。</p>
          </div>
          <div className="form-actions">
            <Button disabled={!selected || !buffer || !dirty || pending === "save"} onClick={() => void save()}>
              <Save aria-hidden="true" />
              {pending === "save" ? "正在保存..." : "保存配置"}
            </Button>
          </div>
        </div>
      ) : null}
      <div className="config-workspace__body">
        <aside className="config-workspace__files" aria-label="配置文件列表">
          <div className="config-workspace__files-heading">
            <h3>配置文件</h3>
            <span>{items.length}</span>
          </div>
          {items.map((file) => {
            const fileBuffer = buffers[file.id];
            const fileDirty = Boolean(fileBuffer && fileBuffer.content !== fileBuffer.original);
            return (
              <button
                key={file.id}
                type="button"
                aria-selected={selected?.id === file.id}
                onClick={() => {
                  setDiagnostics([]);
                  if (buffers[file.id]) {
                    setSelectedId(file.id);
                  } else {
                    void reveal(file);
                  }
                }}
              >
                <FileCode2 aria-hidden="true" />
                <span>
                  <strong>{file.label}</strong>
                  <small>
                    {file.deployPath ?? file.path} · {formatLabel(file.format)}
                  </small>
                </span>
                {fileDirty ? <b className="config-file-dirty" title="有未保存修改">●</b> : null}
                {file.sensitive ? <Lock aria-label="敏感配置" /> : null}
                {!file.editable ? <ShieldAlert aria-label="只读" /> : null}
              </button>
            );
          })}
        </aside>
        <div className="config-workspace__editor">
          {!selected ? <PageState kind="empty" /> : !buffer ? (
            selected.sensitive && !reauthOpen ? (
              <div className="empty-inline">
                <p>该文件是敏感配置，读取前需要重新验证管理员密码。</p>
                <Button tone="primary" onClick={() => setReauthOpen(true)}>验证并读取</Button>
              </div>
            ) : reauthOpen ? (
              <ReauthPanel
                pending={pending === "reauth" || pending === "reveal"}
                error={error}
                onCancel={() => setReauthOpen(false)}
                onSubmit={(password) => void handleReauth(password)}
              />
            ) : (
              <PageState kind="loading" />
            )
          ) : (
            <>
              <header className="config-editor-heading">
                <div>
                  <h3>
                    <code>{selected.deployPath ?? selected.path}</code>
                    {selected.sensitive ? <span className="config-sensitive-badge">敏感</span> : null}
                    {!selected.editable ? <span className="config-readonly-badge">只读</span> : null}
                  </h3>
                  <p>{selected.description}</p>
                </div>
                <div className="form-actions">
                  <Button disabled={!buffer || pending === "validate"} onClick={() => void validate()}>
                    {pending === "validate" ? "正在校验..." : "校验"}
                  </Button>
                  <Button disabled={!buffer} onClick={() => void openVersions()}>
                    <History aria-hidden="true" />版本
                  </Button>
                  {selected.editable ? (
                    <Button
                      tone="primary"
                      disabled={!dirty || pending === "save"}
                      onClick={() => void save()}
                    >
                      <Save aria-hidden="true" />
                      {pending === "save" ? "正在保存..." : "保存"}
                    </Button>
                  ) : null}
                </div>
              </header>
              {error ? <ApiErrorNotice error={error} /> : null}
              <CodeEditor
                value={buffer.content}
                onChange={(content) =>
                  setBuffers((current) => ({
                    ...current,
                    [selected.id]: { ...buffer, content },
                  }))
                }
                language={selected.language}
                format={selected.format}
                readOnly={!selected.editable}
                ariaLabel={`${selected.label} 编辑器`}
                height={height ?? (embedded ? "min(46vh, 520px)" : "min(56vh, 640px)")}
              />
              {diagnostics.length > 0 ? (
                <div className="notice notice--danger config-validation-errors" role="alert">
                  <strong>配置校验未通过</strong>
                  <ul>
                    {diagnostics.map((item, index) => (
                      <li key={`${item.path}-${item.line}-${item.column}-${index}`}>
                        {item.path}:{item.line}:{item.column} · {item.message}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
              {selected.sensitive && grant ? (
                <p className="env-editor-footnote">
                  敏感配置授权有效至 {new Date(grant.expiresAt).toLocaleTimeString("zh-CN")}，离开页面后自动清除明文。
                </p>
              ) : null}
            </>
          )}
        </div>
      </div>
      {versionsOpen && selected ? (
        <div className="modal-backdrop">
          <div className="confirm-dialog config-versions-dialog" role="dialog" aria-modal="true" aria-label={`${selected.label} 版本历史`}>
            <h2>{selected.label} · 版本历史</h2>
            {versions.length === 0 ? <p className="notice">没有历史版本。</p> : (
              <ul className="config-versions-list">
                {versions.map((version) => (
                  <li key={version.id}>
                    <span>
                      <strong>v{version.configVersion}</strong>
                      {version.configVersion === selected.currentVersion ? <em>当前</em> : null}
                    </span>
                    <span>{sourceLabel(version.source)}</span>
                    <time>{new Date(version.createdAt).toLocaleString("zh-CN")}</time>
                    {version.configVersion !== selected.currentVersion && buffer ? (
                      <Button onClick={() => void restoreVersion(version.configVersion)}>恢复</Button>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
            <div className="confirm-dialog__actions">
              <Button disabled={pending === "restore"} onClick={() => setVersionsOpen(false)}>关闭</Button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
