import { useMutation, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { sshCredentialsApi } from "./api";
import { useCursorCollection } from "../shared/useCursorCollection";

export function CredentialsPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const list = useCursorCollection(["ssh-credentials"], (after) => sshCredentialsApi.sshCredentialsList({ limit: 50, after: after ?? undefined }));
  const create = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return sshCredentialsApi.sshCredentialsCreate({ xCSRFToken: auth.csrfToken, createCredentialRequest: { name: name.trim() } });
    },
    onSuccess: async () => {
      setName("");
      setCreating(false);
      await queryClient.invalidateQueries({ queryKey: ["ssh-credentials"] });
    },
  });

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (name.trim() && !create.isPending) await create.mutateAsync().catch(() => undefined);
  }

  return (
    <section className="workspace">
      <div className="workspace-heading">
        <div><h2>SSH 密钥</h2><p>平台只展示公钥。私钥加密保存在服务端，且不会返回客户端。</p></div>
        <Button tone="primary" onClick={() => setCreating((value) => !value)}><Plus aria-hidden="true" />生成密钥</Button>
      </div>
      {creating ? (
        <form className="inline-form" onSubmit={(event) => void submit(event)}>
          <label>密钥名称<input autoFocus required maxLength={80} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：生产节点部署密钥" /></label>
          <div className="form-actions"><Button type="button" onClick={() => setCreating(false)}>取消</Button><Button tone="primary" disabled={!name.trim() || create.isPending}>{create.isPending ? "正在生成..." : "确认生成"}</Button></div>
          {create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}
        </form>
      ) : null}
      {list.isLoading ? <PageState kind="loading" /> : list.isError ? (
        <div className="state-with-action"><ApiErrorNotice error={toNotice(list.error)} /><Button onClick={() => void list.refetch()}>重试</Button></div>
      ) : list.items.length === 0 ? <PageState kind="empty" /> : (
        <div className="data-table-wrap"><table className="data-table"><thead><tr><th>名称</th><th>算法</th><th>指纹</th><th>节点绑定</th><th></th></tr></thead><tbody>
          {list.items.map((credential) => <tr key={credential.id}><td><KeyRound aria-hidden="true" /><strong>{credential.name}</strong></td><td>{credential.algorithm}</td><td><code>{credential.fingerprint}</code></td><td>进入详情查看</td><td><Link className="text-link" to={`/settings/credentials/${credential.id}`}>管理</Link></td></tr>)}
        </tbody></table>{list.hasNextPage ? <div className="pagination-actions"><Button onClick={() => void list.fetchNextPage()}>加载更多</Button></div> : null}</div>
      )}
    </section>
  );
}

export function toNotice(error: unknown): ApiError { return error instanceof Error && "status" in error ? error as ApiError : new ApiError(0, "unexpected_error", error instanceof Error ? error.message : "请求未能完成"); }
