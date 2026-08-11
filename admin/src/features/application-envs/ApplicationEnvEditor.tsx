import { Plus, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { ApiError } from "../../api/http-client";
import { Button } from "../../components/Button";
import { Field, TextInput } from "../../components/form";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { appendAssignment, assignments, parseDotenv, updateAssignment } from "./dotenv";

export function ReauthenticationForm({ title, submitLabel, pending, error, onCancel, onSubmit }: { title: string; submitLabel: string; pending: boolean; error: ApiError | null; onCancel?(): void; onSubmit(password: string): void }) {
  const [password, setPassword] = useState("");
  function submit(event: FormEvent) { event.preventDefault(); onSubmit(password); setPassword(""); }
  return <section className="env-reauth"><h3>{title}</h3><p>本次验证签发 5 分钟临时授权，不会延长当前登录会话。</p><form onSubmit={submit}><Field label="管理员密码"><TextInput required autoComplete="current-password" type="password" value={password} onChange={(event) => setPassword(event.target.value)} /></Field>{error ? <ApiErrorNotice error={error} /> : null}<div className="form-actions">{onCancel ? <Button type="button" disabled={pending} onClick={onCancel}>取消</Button> : null}<Button tone="primary" disabled={pending || !password}>{pending ? "正在验证..." : submitLabel}</Button></div></form></section>;
}

export function StructuredEditor({ document, onChange }: { document: ReturnType<typeof parseDotenv>; onChange(lines: ReturnType<typeof parseDotenv>["lines"]): void }) {
  const rows = assignments(document);
  return <div className="env-structured-editor"><div className="env-table-head"><span>变量名</span><span>值</span><span aria-label="操作"></span></div>{rows.map((row) => <div className="env-key-row" key={row.index}><TextInput aria-label={`${row.key} 的变量名`} value={row.key} onChange={(event) => onChange(updateAssignment(document.lines, row.index, { key: event.target.value }))} /><TextInput aria-label={`${row.key} 的值`} value={row.value} onChange={(event) => onChange(updateAssignment(document.lines, row.index, { value: event.target.value }))} /><Button aria-label={`删除 ${row.key}`} onClick={() => onChange(document.lines.filter((_, index) => index !== row.index))}><Trash2 aria-hidden="true" /></Button></div>)}<Button onClick={() => onChange(appendAssignment(document.lines))}><Plus aria-hidden="true" />增加变量</Button></div>;
}

export function RawEditor({ fileName, content, errors, onChange }: { fileName: string; content: string; errors: ReturnType<typeof parseDotenv>["errors"]; onChange(value: string): void }) {
  const errorId = `${fileName.replaceAll(".", "-")}-errors`;
  return <><div className="env-raw-editor"><pre className="env-line-numbers" aria-hidden="true">{content.split("\n").map((_, index) => index + 1).join("\n")}</pre><textarea aria-label={`${fileName} 原文`} aria-invalid={errors.length > 0} aria-describedby={errors.length > 0 ? errorId : undefined} spellCheck={false} value={content} onChange={(event) => onChange(event.target.value)} /></div>{errors.length > 0 ? <ValidationErrors id={errorId} errors={errors} /> : null}</>;
}

export function ValidationErrors({ id, errors }: { id?: string; errors: ReturnType<typeof parseDotenv>["errors"] }) {
  return <div id={id} className="notice notice--danger env-validation-errors" role="alert"><strong>原文校验未通过</strong><ul>{errors.map((error, index) => <li key={`${error.line}-${error.code}-${index}`}>第 {error.line} 行：{error.message}</li>)}</ul></div>;
}
