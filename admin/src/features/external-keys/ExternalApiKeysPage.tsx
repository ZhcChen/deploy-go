import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Ban, Check, KeyRound, Plus, Settings2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "../../components/Button";
import { Field, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import type { ExternalApiKeyCreatedResponse, ExternalApiKeySummary } from "../../api/generated";
import { useAuth } from "../auth/AuthContext";
import { applicationsApi } from "../applications/api";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { toNotice } from "../shared/toNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { externalKeysApi } from "./api";

export function ExternalApiKeysPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [selectedApplicationIds, setSelectedApplicationIds] = useState<string[]>([]);
  const [created, setCreated] = useState<ExternalApiKeyCreatedResponse | null>(null);
  const [managingKeyId, setManagingKeyId] = useState<string | null>(null);

  const keys = useCursorCollection(["external-api-keys"], (after) => externalKeysApi.externalApiKeysList({ limit: 50, after: after ?? undefined }));
  const applications = useCursorCollection(["applications", "external-key-options"], (after) => applicationsApi.applicationsList({ limit: 50, after: after ?? undefined }));
  const activeApplications = applications.items.filter((application) => application.status === "active");

  const create = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return externalKeysApi.externalApiKeysCreate({
        xCSRFToken: auth.csrfToken,
        createExternalApiKeyRequest: {
          name: name.trim(),
          applicationIds: selectedApplicationIds,
          expiresAt: expiresAt ? new Date(expiresAt).toISOString() : null,
        },
      });
    },
    onSuccess: async (result) => {
      setCreated(result);
      setName("");
      setExpiresAt("");
      setSelectedApplicationIds([]);
      setCreating(false);
      await queryClient.invalidateQueries({ queryKey: ["external-api-keys"] });
    },
  });

  const revoke = useMutation({
    mutationFn: async (apiKey: ExternalApiKeySummary) => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return externalKeysApi.externalApiKeysRevoke({ id: apiKey.id, xCSRFToken: auth.csrfToken });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["external-api-keys"] });
    },
  });

  const updateApplications = useMutation({
    mutationFn: async ({ apiKey, applicationIds }: { apiKey: ExternalApiKeySummary; applicationIds: string[] }) => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return externalKeysApi.externalApiKeysUpdateApplications({
        id: apiKey.id,
        xCSRFToken: auth.csrfToken,
        updateExternalApiKeyApplicationsRequest: { applicationIds },
      });
    },
    onSuccess: async () => {
      setManagingKeyId(null);
      await queryClient.invalidateQueries({ queryKey: ["external-api-keys"] });
    },
  });

  function toggleApplication(applicationId: string) {
    setSelectedApplicationIds((ids) => ids.includes(applicationId) ? ids.filter((id) => id !== applicationId) : [...ids, applicationId]);
  }

  async function submitCreate(event: FormEvent) {
    event.preventDefault();
    await create.mutateAsync().catch(() => undefined);
  }

  function changeStatus(apiKey: ExternalApiKeySummary) {
    if (apiKey.status === "active" && !window.confirm("吊销后该 Key 立即失效且不能恢复，确定继续吗？")) return;
    revoke.mutate(apiKey);
  }

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>对外 API Key</h2><p>为外部系统、CI 或 Agent 创建只读应用列表与部署操作凭据；明文只在创建时显示一次，不能再次查看。</p></div><Button tone="primary" onClick={() => { setCreating(true); setCreated(null); }}><Plus aria-hidden="true" />创建 API Key</Button></div>

    {created ? <div className="notice notice--warning"><strong>API Key 已创建，明文只显示这一次</strong><small>{created.id} · 请立即复制保存；关闭或刷新页面后无法再次查看。</small><ClipboardFallback value={created.token} label="复制 API Key" /><div className="form-actions"><Button onClick={() => setCreated(null)}>我已保存，关闭提示</Button></div></div> : null}

    {creating ? <form className="inline-form" onSubmit={(event) => void submitCreate(event)}>
      <Field label="Key 名称"><TextInput autoFocus required minLength={1} maxLength={80} disabled={create.isPending} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：外部 CI 部署" /></Field>
      <Field label="过期时间（可选）"><TextInput type="datetime-local" disabled={create.isPending} value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} /><small>留空表示永不过期；过期后外部调用会被拒绝。</small></Field>
      <div className="form-field"><span>可部署应用</span><ul className="grant-list">{activeApplications.map((application) => {
        const selected = selectedApplicationIds.includes(application.id);
        return <li key={application.id}><button type="button" aria-pressed={selected} disabled={create.isPending || activeApplications.length === 0} onClick={() => toggleApplication(application.id)}><span className={`grant-check${selected ? " is-checked" : ""}`}>{selected ? <Check aria-hidden="true" /> : null}</span><span><strong>{application.name}</strong><small>{application.slug}</small></span><em>{selected ? "已选择" : "未选择"}</em></button></li>;
      })}</ul>{activeApplications.length === 0 ? <small>没有启用中的应用，请先在应用页面创建。</small> : <small>至少选择一个应用；API Key 只能部署绑定应用。</small>}</div>
      {applications.hasNextPage ? <div className="pagination-actions"><Button disabled={applications.isFetchingNextPage} onClick={() => void applications.fetchNextPage()}>{applications.isFetchingNextPage ? "正在加载..." : "加载更多应用"}</Button></div> : null}
      {create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setName(""); setExpiresAt(""); setSelectedApplicationIds([]); }}>取消</Button><Button tone="primary" disabled={create.isPending || !name.trim() || selectedApplicationIds.length === 0}>{create.isPending ? "正在创建..." : "创建 Key"}</Button></div>
    </form> : null}

    {keys.isLoading ? <PageState kind="loading" /> : keys.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(keys.error)} /><Button onClick={() => void keys.refetch()}>重试</Button></div> : keys.items.length === 0 ? <PageState kind="empty" /> : <div className="data-table-wrap"><table className="data-table"><thead><tr><th>Key</th><th>状态</th><th>绑定应用</th><th>最后使用</th><th>过期时间</th><th></th></tr></thead><tbody>{keys.items.map((apiKey) => {
      const state = keyState(apiKey);
      return <ApiKeyRow key={apiKey.id} apiKey={apiKey} state={state} applications={activeApplications} applicationsLoading={applications.isLoading} managing={managingKeyId === apiKey.id} onManage={() => setManagingKeyId((id) => id === apiKey.id ? null : apiKey.id)} onRevoke={() => changeStatus(apiKey)} revokePending={revoke.isPending && revoke.variables?.id === apiKey.id} onSave={async (applicationIds) => { await updateApplications.mutateAsync({ apiKey, applicationIds }).catch(() => undefined); }} savePending={updateApplications.isPending && updateApplications.variables?.apiKey.id === apiKey.id} saveError={updateApplications.variables?.apiKey.id === apiKey.id ? updateApplications.error : null} />;
    })}</tbody></table></div>}
    {keys.hasNextPage ? <div className="pagination-actions"><Button disabled={keys.isFetchingNextPage} onClick={() => void keys.fetchNextPage()}>{keys.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}
  </section>;
}

function keyState(apiKey: ExternalApiKeySummary) {
  if (apiKey.status !== "active") return { label: "已吊销", tone: "disabled" };
  if (apiKey.expiresAt && Date.parse(apiKey.expiresAt) <= Date.now()) return { label: "已过期", tone: "disabled" };
  return { label: "启用", tone: "online" };
}

function ApiKeyRow({ apiKey, state, applications, applicationsLoading, managing, onManage, onRevoke, revokePending, onSave, savePending, saveError }: {
  apiKey: ExternalApiKeySummary;
  state: { label: string; tone: string };
  applications: Array<{ id: string; name: string; slug: string; status: string }>;
  applicationsLoading: boolean;
  managing: boolean;
  onManage(): void;
  onRevoke(): void;
  revokePending: boolean;
  onSave(applicationIds: string[]): Promise<void>;
  savePending: boolean;
  saveError: unknown;
}) {
  return <>
    <tr>
      <td><KeyRound aria-hidden="true" /><strong>{apiKey.name}</strong><small>创建于 {new Date(apiKey.createdAt).toLocaleString("zh-CN")}</small></td>
      <td><span className={`status-badge status-badge--${state.tone}`}>{state.label}</span></td>
      <td>{apiKey.applicationIds.length} 个应用</td>
      <td>{apiKey.lastUsedAt ? new Date(apiKey.lastUsedAt).toLocaleString("zh-CN") : "从未使用"}</td>
      <td>{apiKey.expiresAt ? new Date(apiKey.expiresAt).toLocaleString("zh-CN") : "永不过期"}</td>
      <td><div className="resource-actions"><Button disabled={apiKey.status !== "active"} onClick={onManage}><Settings2 aria-hidden="true" />{managing ? "收起" : "管理应用"}</Button><Button tone="danger" disabled={apiKey.status !== "active" || revokePending} onClick={onRevoke}><Ban aria-hidden="true" />{revokePending ? "吊销中..." : "吊销"}</Button></div></td>
    </tr>
    {managing ? <tr><td colSpan={6}><div className="inline-form"><div className="section-heading"><div><h4>绑定应用</h4><p>仅显示启用中的应用；保存后 API Key 只能部署勾选的应用，至少保留一个，停用请吊销 Key。</p></div></div><KeyApplicationPicker apiKey={apiKey} applications={applications} loading={applicationsLoading} onSave={onSave} pending={savePending} error={saveError} /></div></td></tr> : null}
  </>;
}

function KeyApplicationPicker({ apiKey, applications, loading, onSave, pending, error }: {
  apiKey: ExternalApiKeySummary;
  applications: Array<{ id: string; name: string; slug: string; status: string }>;
  loading: boolean;
  onSave(applicationIds: string[]): Promise<void>;
  pending: boolean;
  error: unknown;
}) {
  const boundIds = new Set(apiKey.applicationIds);
  const [selectedIds, setSelectedIds] = useState(apiKey.applicationIds.filter((id) => applications.some((application) => application.id === id)));

  function toggle(applicationId: string) {
    setSelectedIds((ids) => ids.includes(applicationId) ? ids.filter((id) => id !== applicationId) : [...ids, applicationId]);
  }

  if (loading) return <PageState kind="loading" />;
  return <><ul className="grant-list">{applications.map((application) => {
    const selected = selectedIds.includes(application.id);
    const originallyBound = boundIds.has(application.id);
    return <li key={application.id}><button type="button" aria-pressed={selected} disabled={pending} onClick={() => toggle(application.id)}><span className={`grant-check${selected ? " is-checked" : ""}`}>{selected ? <Check aria-hidden="true" /> : null}</span><span><strong>{application.name}</strong><small>{application.slug}</small></span><em>{selected ? "已选择" : originallyBound ? "保存后移除" : "未选择"}</em></button></li>;
  })}</ul>{applications.length === 0 ? <p className="form-help">没有启用中的应用，该 Key 当前无法部署。</p> : null}{error ? <ApiErrorNotice error={toNotice(error)} /> : null}<div className="form-actions"><Button disabled={pending || selectedIds.length === 0} onClick={() => void onSave(selectedIds)}>{pending ? "保存中..." : "保存应用"}</Button></div></>;
}
