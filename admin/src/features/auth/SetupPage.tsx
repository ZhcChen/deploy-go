import { useState, type FormEvent } from "react";
import { Navigate } from "react-router-dom";
import { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { Field, TextInput } from "../../components/form";
import { useAuth } from "./AuthContext";
import { AuthLayout } from "./AuthLayout";
import { ApiErrorNotice, type ErrorNoticeValue } from "../errors/ApiErrorNotice";

export function SetupPage() {
  const auth = useAuth();
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);
  const [error, setError] = useState<ErrorNoticeValue | null>(null);

  if (complete || auth.status === "anonymous") return <Navigate replace to="/login" />;
  if (auth.status !== "booting" && auth.status !== "setup_required") return <Navigate replace to="/overview" />;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (pending) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    setPending(true);
    setError(null);
    try {
      await auth.setup({
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
      <div><h1>创建管理员</h1><p>全新实例首次访问时创建唯一管理员，完成后登录入口自动关闭。</p></div>
      {error ? <ApiErrorNotice error={error} /> : null}
      <form className="auth-form" onSubmit={submit}>
        <Field label="登录账号"><TextInput name="username" autoComplete="username" required autoFocus /></Field>
        <Field label="显示名称"><TextInput name="displayName" autoComplete="name" /></Field>
        <Field label="邮箱"><TextInput name="email" type="email" autoComplete="email" /></Field>
        <Field label="初始密码"><TextInput name="password" type="password" autoComplete="new-password" minLength={8} required /></Field>
        <Button tone="primary" disabled={pending} type="submit">{pending ? "初始化中" : "完成初始化"}</Button>
      </form>
      <p className="auth-help">初始化必须在系统仍为空库时完成，初始化后无法再次进入。</p>
    </AuthLayout>
  );
}
