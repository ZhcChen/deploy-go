import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import type { RuntimeSettings } from "../../api/generated/models/RuntimeSettings";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { settingsApi } from "./api";

export function SettingsPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const settings = useQuery({
    queryKey: ["runtime-settings"],
    queryFn: () => settingsApi.settingsShow(),
  });
  const [draft, setDraft] = useState<RuntimeSettings | null>(null);
  const form = draft ?? settings.data ?? null;
  const dirty = Boolean(draft && settings.data && JSON.stringify(draft) !== JSON.stringify(settings.data));
  useUnsavedChanges(dirty);
  const update = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !form) throw new Error("缺少必要的安全上下文");
    return settingsApi.settingsUpdate({ xCSRFToken: auth.csrfToken, runtimeSettings: form });
  }, onSuccess: (saved) => { queryClient.setQueryData(["runtime-settings"], saved); setDraft(null); } });
  async function submit(event: FormEvent) { event.preventDefault(); await update.mutateAsync().catch(() => undefined); }
  if (settings.isLoading) return <PageState kind="loading" />;
  if (settings.isError || !form) return <div className="state-with-action"><ApiErrorNotice error={toNotice(settings.error)} /><Button onClick={() => void settings.refetch()}>重试</Button></div>;
  return <section className="workspace settings-page">
    <div className="workspace-heading"><div><h2>系统设置</h2><p>控制本实例的部署并发和日志保留边界。</p></div></div>
    <form className="settings-form" onSubmit={(event) => void submit(event)}>
      <label>最大并发部署数<input type="number" required min="1" max="64" disabled={update.isPending} value={form.maxConcurrentDeployments} onChange={(event) => setDraft({ ...form, maxConcurrentDeployments: Number(event.target.value) })} /><small>允许范围 1 至 64。</small></label>
      <label>单次日志上限（MiB）<input type="number" required min="1" max="1024" disabled={update.isPending} value={form.maxLogBytes / 1024 / 1024} onChange={(event) => setDraft({ ...form, maxLogBytes: Number(event.target.value) * 1024 * 1024 })} /><small>达到上限后执行结果仍保留，但不再追加输出。</small></label>
      <label>日志保留天数<input type="number" required min="1" max="3650" disabled={update.isPending} value={form.logRetentionDays} onChange={(event) => setDraft({ ...form, logRetentionDays: Number(event.target.value) })} /><small>过期后只清理输出，不删除部署历史。</small></label>
      {update.error ? <ApiErrorNotice error={toNotice(update.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={!dirty || update.isPending} onClick={() => setDraft(null)}>丢弃草稿</Button><Button tone="primary" disabled={!dirty || update.isPending}>{update.isPending ? "正在保存..." : "保存设置"}</Button></div>
    </form>
  </section>;
}
