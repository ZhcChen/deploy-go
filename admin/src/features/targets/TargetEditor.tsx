import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import type { DeploymentTargetResponse } from "../../api/generated/models/DeploymentTargetResponse";
import type { SaveTargetRequest } from "../../api/generated/models/SaveTargetRequest";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { Field, Select, TextArea, TextInput } from "../../components/form";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { applicationSourcesApi, deploymentTargetsApi } from "../applications/api";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";

interface TargetDraft {
  nodeId: string;
  executionMode: string;
  scriptPath: string;
  parameterSchema: string;
  timeoutSeconds: string;
  verificationConfig: string;
  secretReferences: string;
  privilegedRelease: boolean;
  privilegedReleaseConfirmed: boolean;
}

const initialDraft: TargetDraft = {
  nodeId: "", executionMode: "script", scriptPath: "/srv/apps/example/deploy.sh",
  parameterSchema: JSON.stringify({ type: "object", properties: {}, additionalProperties: false }, null, 2),
  timeoutSeconds: "900",
  verificationConfig: JSON.stringify({ type: "http", path: "/healthz", expected_status: 200, timeout_ms: 5000 }, null, 2),
  secretReferences: "",
  privilegedRelease: false,
  privilegedReleaseConfirmed: false,
};

function fromTarget(target: DeploymentTargetResponse): TargetDraft {
  return {
    nodeId: target.nodeId,
    executionMode: target.executionMode,
    scriptPath: target.scriptPath,
    parameterSchema: JSON.stringify(target.parameterSchema, null, 2),
    timeoutSeconds: String(target.timeoutSeconds),
    verificationConfig: JSON.stringify(target.verificationConfig, null, 2),
    secretReferences: target.secretFileReferences.map((item) => `${item.environmentKey}=${item.filePath}`).join("\n"),
    privilegedRelease: target.privilegedRelease,
    privilegedReleaseConfirmed: target.privilegedRelease,
  };
}

export function TargetEditor({ applicationId, nodes, target, hasMoreNodes, loadingMoreNodes, onLoadMoreNodes, onDiscard, onSaved }: { applicationId: string; nodes: NodeResponse[]; target?: DeploymentTargetResponse; hasMoreNodes?: boolean; loadingMoreNodes?: boolean; onLoadMoreNodes?(): void; onDiscard(): void; onSaved?(target: DeploymentTargetResponse): void }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState(() => target ? fromTarget(target) : initialDraft);
  const source = useQuery({ queryKey: ["application-source", applicationId], queryFn: () => applicationSourcesApi.applicationSourceShow({ applicationId }), retry: false, enabled: draft.executionMode === "two_stage" });
  const initial = target ? fromTarget(target) : initialDraft;
  useUnsavedChanges(JSON.stringify(draft) !== JSON.stringify(initial));
  const [parseError, setParseError] = useState<string | null>(null);
  const save = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    const request = parseDraft(draft, target?.version);
    return target
      ? deploymentTargetsApi.deploymentTargetsUpdate({ id: target.id, xCSRFToken: auth.csrfToken, saveTargetRequest: request })
      : deploymentTargetsApi.deploymentTargetsCreate({ applicationId, xCSRFToken: auth.csrfToken, saveTargetRequest: request });
  }, onSuccess: async (saved) => { await queryClient.invalidateQueries({ queryKey: ["deployment-targets", applicationId] }); onSaved?.(saved); } });
  async function submit(event: FormEvent) {
    event.preventDefault();
    setParseError(null);
    try { parseDraft(draft, target?.version); } catch (error) { setParseError(error instanceof Error ? error.message : "配置格式不正确"); return; }
    await save.mutateAsync().catch(() => undefined);
  }
  return <form className="target-form" onSubmit={(event) => void submit(event)}>
    <Field label="节点"><Select required value={draft.nodeId} onChange={(event) => setDraft({
      ...draft,
      nodeId: event.target.value,
      privilegedReleaseConfirmed: Boolean(event.target.value === target?.nodeId && target?.privilegedRelease),
    })}><option value="">选择已在线节点</option>{nodes.filter((node) => node.status === "online" || node.id === draft.nodeId).map((node) => <option key={node.id} value={node.id}>{node.name} · {node.host}</option>)}</Select>{hasMoreNodes ? <Button type="button" disabled={loadingMoreNodes} onClick={onLoadMoreNodes}>{loadingMoreNodes ? "正在加载..." : "加载更多节点"}</Button> : null}</Field>
    <Field label="执行模式"><Select required value={draft.executionMode} onChange={(event) => {
      const twoStage = event.target.value === "two_stage";
      setDraft({
        ...draft,
        executionMode: event.target.value,
        privilegedRelease: twoStage && draft.privilegedRelease,
        privilegedReleaseConfirmed: twoStage && draft.privilegedReleaseConfirmed,
      });
    }}><option value="script">单脚本模式</option><option value="two_stage">两阶段模式（prepare + release）</option></Select></Field>
    <Field label={draft.executionMode === "two_stage" ? "发布脚本路径（Agent 固定执行 make deploy-go-release 的占位路径）" : "脚本绝对路径"} className="form-span"><TextInput required value={draft.scriptPath} onChange={(event) => setDraft({ ...draft, scriptPath: event.target.value })} /></Field>
    {draft.executionMode === "two_stage" ? <p className="notice form-span">两阶段模式要求应用已配置并固定 Git 来源，且目标 Agent 协议版本不低于 2。<code>release-version</code> 由平台自动生成；请通过 <code>modules.x-options</code> 声明可选模块。</p> : null}
    {draft.executionMode === "two_stage" ? <section className="notice form-span target-privileged-release">
      <label className="checkbox-field">
        <input type="checkbox" checked={draft.privilegedRelease} onChange={(event) => setDraft({ ...draft, privilegedRelease: event.target.checked, privilegedReleaseConfirmed: false })} />
        <span><strong>使用 Agent 原生特权 release</strong><small>release 将由目标节点 root executor 执行固定 Make target；prepare 仍使用低权限 runner。</small></span>
      </label>
      {draft.privilegedRelease ? <div className="target-privileged-release__confirmation">
        <dl className="definition-grid">
          <div><dt>仓库</dt><dd><code>{source.data?.repositoryUrl ?? "来源尚未加载"}</code></dd></div>
          <div><dt>固定分支</dt><dd><code>{source.data?.deploymentBranch ?? "尚未固定"}</code></dd></div>
          <div><dt>目标节点</dt><dd><code>{(nodes.find((node) => node.id === draft.nodeId)?.name ?? draft.nodeId) || "尚未选择"}</code></dd></div>
        </dl>
        <label className="checkbox-field checkbox-field--danger">
          <input type="checkbox" checked={draft.privilegedReleaseConfirmed} onChange={(event) => setDraft({ ...draft, privilegedReleaseConfirmed: event.target.checked })} />
          <span>我确认该仓库和固定分支的写入者将获得目标节点 root 发布能力</span>
        </label>
      </div> : null}
    </section> : null}
    <Field label="超时秒数"><TextInput required type="number" min="1" max="86400" value={draft.timeoutSeconds} onChange={(event) => setDraft({ ...draft, timeoutSeconds: event.target.value })} /></Field>
    <Field label="敏感文件引用" hint={"每行 `ENV_KEY=/absolute/path`，平台只传路径，不读取内容。"}><TextArea rows={4} value={draft.secretReferences} onChange={(event) => setDraft({ ...draft, secretReferences: event.target.value })} placeholder={"DEPLOY_TOKEN_FILE=/srv/secrets/app/token\nENV_FILE=/srv/secrets/app/.env"} /></Field>
    <Field label="参数 JSON Schema" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.parameterSchema} onChange={(event) => setDraft({ ...draft, parameterSchema: event.target.value })} /></Field>
    <Field label="部署后验证配置" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.verificationConfig} onChange={(event) => setDraft({ ...draft, verificationConfig: event.target.value })} /></Field>
    {parseError ? <div className="notice notice--danger form-span" role="alert"><strong>{parseError}</strong></div> : null}
    {save.error ? <div className="form-span"><ApiErrorNotice error={toNotice(save.error)} /></div> : null}
    <div className="form-actions form-span"><Button type="button" onClick={onDiscard}>丢弃草稿</Button><Button tone="primary" disabled={save.isPending}>{save.isPending ? "正在保存..." : "保存目标"}</Button></div>
  </form>;
}

function parseDraft(draft: TargetDraft, version?: number): SaveTargetRequest {
  let parameterSchema: unknown;
  let verificationConfig: unknown;
  try { parameterSchema = JSON.parse(draft.parameterSchema); } catch { throw new Error("参数 JSON Schema 不是有效 JSON"); }
  try { verificationConfig = JSON.parse(draft.verificationConfig); } catch { throw new Error("验证配置不是有效 JSON"); }
  if (!isObject(parameterSchema) || !isObject(verificationConfig)) throw new Error("Schema 和验证配置必须是 JSON object");
  const secretFileReferences = draft.secretReferences.split("\n").map((line) => line.trim()).filter(Boolean).map((line) => {
    const separator = line.indexOf("=");
    if (separator <= 0 || separator === line.length - 1) throw new Error("敏感文件引用必须使用 ENV_KEY=/absolute/path 格式");
    return { environmentKey: line.slice(0, separator).trim(), filePath: line.slice(separator + 1).trim() };
  });
  if (draft.privilegedRelease && !draft.privilegedReleaseConfirmed) throw new Error("开启 Agent 原生特权 release 前必须确认 root 信任边界");
  return { nodeId: draft.nodeId, executionMode: draft.executionMode, scriptPath: draft.scriptPath.trim(), parameterSchema, timeoutSeconds: Number(draft.timeoutSeconds), verificationConfig, secretFileReferences, privilegedRelease: draft.privilegedRelease, privilegedReleaseConfirmed: draft.privilegedReleaseConfirmed, version };
}

function isObject(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
