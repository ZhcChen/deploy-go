import { useState, type FormEvent } from "react";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { Button } from "../../components/Button";
import { ApiError } from "../../api/http-client";
import { safeReturnPath } from "../../routes/safeReturnPath";
import { useAuth } from "./AuthContext";
import { AuthLayout } from "./AuthLayout";
import { ApiErrorNotice, type ErrorNoticeValue } from "../errors/ApiErrorNotice";

export function LoginPage() {
  const auth = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorNoticeValue | null>(null);

  if (auth.status === "setup_required") return <Navigate replace to="/setup" />;
  if (auth.status === "authenticated") {
    return <Navigate replace to={safeReturnPath((location.state as { from?: unknown } | null)?.from)} />;
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (pending) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    setPending(true);
    setError(null);
    try {
      await auth.login({ username: String(data.get("username") ?? "").trim(), password: String(data.get("password") ?? "") });
      form.reset();
      navigate(safeReturnPath((location.state as { from?: unknown } | null)?.from), { replace: true });
    } catch (cause) {
      setError({
        message: cause instanceof ApiError && cause.status === 401 ? "账号或密码不正确" : cause instanceof Error ? cause.message : "登录失败",
        requestId: cause instanceof ApiError ? cause.requestId : undefined,
      });
    } finally {
      setPending(false);
    }
  }

  return (
    <AuthLayout context="部署控制服务">
      <div><h1>登录</h1><p>使用管理员分配的账号进入控制台。</p></div>
      {auth.sessionExpired ? <div className="notice">会话已失效，请重新登录。</div> : null}
      {error ? <ApiErrorNotice error={error} /> : null}
      <form className="auth-form" onSubmit={submit}>
        <label>账号或邮箱<input name="username" autoComplete="username" required autoFocus /></label>
        <label>密码<input name="password" type="password" autoComplete="current-password" required /></label>
        <Button tone="primary" disabled={pending} type="submit">{pending ? "登录中" : "登录"}</Button>
      </form>
      <p className="auth-help">账号由唯一管理员创建和分配。</p>
    </AuthLayout>
  );
}
