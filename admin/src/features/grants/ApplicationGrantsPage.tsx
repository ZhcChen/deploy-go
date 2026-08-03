import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, UserRound } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { applicationsApi, grantsApi, grantUsersApi } from "../applications/api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";

export function ApplicationGrantsPage() {
  const [userId, setUserId] = useState("");
  const users = useCursorCollection(["users", "grant-options"], (after) => grantUsersApi.usersList({ limit: 50, after: after ?? undefined }));
  const ordinaryUsers = users.items.filter((user) => user.identity === "user");
  const selectedUser = ordinaryUsers.find((user) => user.id === userId);
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>应用授权</h2><p>普通用户只能查看和操作管理员明确分配的应用。</p></div></div>
    <div className="grant-layout"><aside className="user-picker"><h3>普通用户</h3>{users.isLoading ? <PageState kind="loading" /> : users.isError ? <ApiErrorNotice error={toNotice(users.error)} /> : ordinaryUsers.length === 0 ? <PageState kind="empty" /> : <>{ordinaryUsers.map((user) => <button className={userId === user.id ? "is-active" : ""} key={user.id} type="button" onClick={() => setUserId(user.id)}><UserRound aria-hidden="true" /><span><strong>{user.displayName}</strong><small>@{user.username} · {user.status === "active" ? "启用" : "停用"}</small></span></button>)}{users.hasNextPage ? <Button onClick={() => void users.fetchNextPage()}>加载更多用户</Button> : null}</>}</aside>
      <div className="grant-panel">{selectedUser ? <UserGrantPanel key={selectedUser.id} userId={selectedUser.id} userActive={selectedUser.status === "active"} /> : <div className="selection-empty"><UserRound aria-hidden="true" /><p>选择一个普通用户管理应用授权。</p></div>}</div>
    </div>
  </section>;
}

function UserGrantPanel({ userId, userActive }: { userId: string; userActive: boolean }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const applications = useCursorCollection(["applications", "grant-options"], (after) => applicationsApi.applicationsList({ limit: 50, after: after ?? undefined }));
  const grants = useCursorCollection(["application-grants", userId], (after) => grantsApi.grantsList({ userId, limit: 50, after: after ?? undefined }));
  const { fetchNextPage: fetchNextGrantPage, hasNextPage: hasNextGrantPage, isFetchingNextPage: isFetchingNextGrantPage } = grants;
  useEffect(() => {
    if (hasNextGrantPage && !isFetchingNextGrantPage) void fetchNextGrantPage();
  }, [fetchNextGrantPage, hasNextGrantPage, isFetchingNextGrantPage]);
  const grantedIds = new Set(grants.items.map((grant) => grant.applicationId));
  const change = useMutation({ mutationFn: async ({ targetUserId, applicationId, granted }: { targetUserId: string; applicationId: string; granted: boolean }) => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    if (granted) await grantsApi.grantsRevoke({ userId: targetUserId, applicationId, xCSRFToken: auth.csrfToken });
    else await grantsApi.grantsGrant({ userId: targetUserId, applicationId, xCSRFToken: auth.csrfToken });
  }, onSuccess: async (_, variables) => { await queryClient.invalidateQueries({ queryKey: ["application-grants", variables.targetUserId] }); } });
  if (applications.isLoading || grants.isLoading) return <PageState kind="loading" />;
  if (applications.isError || grants.isError) return <div className="state-with-action"><ApiErrorNotice error={toNotice(applications.error ?? grants.error)} /><Button onClick={() => { void applications.refetch(); void grants.refetch(); }}>重试</Button></div>;
  return <><div className="section-heading"><div><h3>已分配应用</h3><p>{userActive ? "勾选变化会立即保存，不会创建角色。" : "用户已停用，只能撤销既有授权。"}</p></div><span>{hasNextGrantPage ? `已加载 ${grantedIds.size} 个授权` : `${grantedIds.size} 个应用`}</span></div>{applications.items.length === 0 ? <PageState kind="empty" /> : <ul className="grant-list">{applications.items.map((application) => {
    const granted = grantedIds.has(application.id);
    const pending = change.isPending && change.variables?.applicationId === application.id;
    const checking = hasNextGrantPage || isFetchingNextGrantPage;
    return <li key={application.id}><button type="button" disabled={checking || change.isPending || (!granted && (!userActive || application.status !== "active"))} aria-pressed={granted} onClick={() => change.mutate({ targetUserId: userId, applicationId: application.id, granted })}><span className={`grant-check${granted ? " is-checked" : ""}`}>{granted ? <Check aria-hidden="true" /> : null}</span><span><strong>{application.name}</strong><small>{application.slug}{application.status !== "active" ? " · 已归档" : ""}</small></span><em>{pending ? "保存中..." : checking ? "核对中..." : granted ? "已分配" : "未分配"}</em></button></li>;
  })}</ul>}{change.error ? <ApiErrorNotice error={toNotice(change.error)} /> : null}<div className="pagination-actions">{applications.hasNextPage ? <Button onClick={() => void applications.fetchNextPage()}>加载更多应用</Button> : null}</div></>;
}
