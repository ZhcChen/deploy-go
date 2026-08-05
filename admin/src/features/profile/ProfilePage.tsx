import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bell, UserRound } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { UserPreferencesResponse } from "../../api/generated/models/UserPreferencesResponse";
import { Button } from "../../components/Button";
import { Field, Select, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { profileApi } from "./api";

export function ProfilePage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const profile = useQuery({ queryKey: ["profile"], queryFn: () => profileApi.authProfile() });
  const preferences = useQuery({ queryKey: ["preferences"], queryFn: () => profileApi.authPreferences() });
  const [displayName, setDisplayName] = useState<string | null>(null);
  const [preferenceDraft, setPreferenceDraft] = useState<UserPreferencesResponse | null>(null);
  const preferenceForm = preferenceDraft ?? preferences.data ?? null;
  const profileDirty = profile.data ? displayName !== null && displayName !== profile.data.displayName : false;
  const preferencesDirty = Boolean(preferenceDraft && preferences.data && JSON.stringify(preferenceDraft) !== JSON.stringify(preferences.data));
  useUnsavedChanges(profileDirty || preferencesDirty);
  const updateProfile = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || displayName === null) throw new Error("缺少必要的安全上下文");
    return profileApi.authUpdateProfile({ xCSRFToken: auth.csrfToken, updateProfileRequest: { displayName: displayName.trim() } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["profile"], saved); setDisplayName(null); auth.applyUser(saved); } });
  const updatePreferences = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !preferenceForm) throw new Error("缺少必要的安全上下文");
    return profileApi.authUpdatePreferences({ xCSRFToken: auth.csrfToken, updateUserPreferencesRequest: preferenceForm });
  }, onSuccess: (saved) => { queryClient.setQueryData(["preferences"], saved); setPreferenceDraft(null); } });
  async function saveProfile(event: FormEvent) { event.preventDefault(); await updateProfile.mutateAsync().catch(() => undefined); }
  async function savePreferences(event: FormEvent) { event.preventDefault(); await updatePreferences.mutateAsync().catch(() => undefined); }
  if (profile.isLoading || preferences.isLoading) return <PageState kind="loading" />;
  if (profile.isError || preferences.isError || !profile.data || !preferenceForm) return <div className="state-with-action"><ApiErrorNotice error={toNotice(profile.error ?? preferences.error)} /><Button onClick={() => { void profile.refetch(); void preferences.refetch(); }}>重试</Button></div>;
  return <section className="workspace profile-page">
    <div className="profile-identity"><span className="profile-avatar"><UserRound aria-hidden="true" /></span><div><h2>{profile.data.displayName}</h2><p>@{profile.data.username} · {profile.data.email || "未设置邮箱"}</p></div><span className="status-badge">{profile.data.identity === "administrator" ? "管理员" : "普通用户"}</span></div>
    <section className="profile-section"><div className="section-heading"><div><h3>个人资料</h3><p>显示名称会同步到所有已登录客户端。</p></div></div><form className="settings-form" onSubmit={(event) => void saveProfile(event)}>
      <Field label="显示名称"><TextInput required maxLength={120} disabled={updateProfile.isPending} value={displayName ?? profile.data.displayName} onChange={(event) => setDisplayName(event.target.value)} /></Field>
      {updateProfile.error ? <ApiErrorNotice error={toNotice(updateProfile.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={!profileDirty || updateProfile.isPending} onClick={() => setDisplayName(null)}>丢弃草稿</Button><Button tone="primary" disabled={!profileDirty || updateProfile.isPending}>保存资料</Button></div>
    </form></section>
    <section className="profile-section"><div className="section-heading"><div><h3><Bell aria-hidden="true" />通知与显示</h3><p>偏好保存在服务端，并在其他客户端恢复。</p></div></div><form className="preference-form" onSubmit={(event) => void savePreferences(event)}>
      <Toggle label="部署失败" checked={preferenceForm.notifyDeploymentFailed} disabled={updatePreferences.isPending} onChange={(value) => setPreferenceDraft({ ...preferenceForm, notifyDeploymentFailed: value })} />
      <Toggle label="部署完成" checked={preferenceForm.notifyDeploymentCompleted} disabled={updatePreferences.isPending} onChange={(value) => setPreferenceDraft({ ...preferenceForm, notifyDeploymentCompleted: value })} />
      <Toggle label="节点异常" checked={preferenceForm.notifyNodeUnhealthy} disabled={updatePreferences.isPending} onChange={(value) => setPreferenceDraft({ ...preferenceForm, notifyNodeUnhealthy: value })} />
      <Toggle label="默认跟随部署日志" checked={preferenceForm.followLogs} disabled={updatePreferences.isPending} onChange={(value) => setPreferenceDraft({ ...preferenceForm, followLogs: value })} />
      <Field className="preference-select" label="时间格式"><Select disabled={updatePreferences.isPending} value={preferenceForm.timeFormat} onChange={(event) => setPreferenceDraft({ ...preferenceForm, timeFormat: event.target.value })}><option value="24h">24 小时</option><option value="12h">12 小时</option></Select></Field>
      {updatePreferences.error ? <ApiErrorNotice error={toNotice(updatePreferences.error)} /> : null}<div className="form-actions"><Button type="button" disabled={!preferencesDirty || updatePreferences.isPending} onClick={() => setPreferenceDraft(null)}>丢弃草稿</Button><Button tone="primary" disabled={!preferencesDirty || updatePreferences.isPending}>保存偏好</Button></div>
    </form></section>
  </section>;
}

function Toggle({ label, checked, disabled, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange(value: boolean): void }) {
  return <label className="toggle-row"><span>{label}</span><input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} /></label>;
}
