import { useState, type FormEvent } from "react";
import { Navigate } from "react-router-dom";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { useAuth } from "./AuthContext";
import { AuthLayout } from "./AuthLayout";
import { ApiErrorNotice, type ErrorNoticeValue } from "../errors/ApiErrorNotice";

export function SetupPage() {
  const auth = useAuth();
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);
  const [error, setError] = useState<ErrorNoticeValue | null>(null);

  if (complete || auth.status === "anonymous") return <Navigate replace to="/login" />;
  if (auth.status === "setup_disabled") {
    return (
      <AuthLayout context="首次初始化">
        <div><h1>初始化未启用</h1><p>控制服务尚未配置一次性 setup token。配置并重启服务后再继续。</p></div>
        <Button tone="primary" onClick={() => void auth.retry()}>重新检查</Button>
      </AuthLayout>
    );
  }
  if (auth.status !== "booting" && auth.status !== "setup_required") return <Navigate replace to="/overview" />;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (pending) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    setPending(true);
    setError(null);
    try {
      await auth.setup(String(data.get("token") ?? ""), {
        username: String(data.get("username") ?? "").trim(),
        displayName: String(data.get("displayName") ?? "").trim() || undefined,
        email: String(data.get("email") ?? "").trim() || undefined,
        password: String(data.get("password") ?? ""),
      });
      form.reset();
      setComplete(true);
    } catch (cause) {
      setError({
        message: cause instanceof ApiError ? cause.message : "初始化失败",
        requestId: cause instanceof ApiError ? cause.requestId : undefined,
      });
    } finally {
      setPending(false);
    }
  }

  return (
    <AuthLayout context="首次初始化">
      <div><h1>创建管理员</h1><p>使用部署时提供的一次性 setup token 初始化唯一管理员。</p></div>
      {error ? <ApiErrorNotice error={error} /> : null}
      <form className="auth-form" onSubmit={submit}>
        <label>Setup Token<input name="token" type="password" autoComplete="off" required autoFocus /></label>
        <label>登录账号<input name="username" autoComplete="username" required /></label>
        <label>显示名称<input name="displayName" autoComplete="name" /></label>
        <label>邮箱<input name="email" type="email" autoComplete="email" /></label>
        <label>初始密码<input name="password" type="password" autoComplete="new-password" minLength={8} required /></label>
        <Button tone="primary" disabled={pending} type="submit">{pending ? "初始化中" : "完成初始化"}</Button>
      </form>
      <p className="auth-help">Token 仅用于本次提交，不会保存到浏览器。</p>
    </AuthLayout>
  );
}
