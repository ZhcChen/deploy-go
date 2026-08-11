import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import type { DeploymentTargetResponse } from "../../api/generated/models/DeploymentTargetResponse";
import type { SaveTargetRequest } from "../../api/generated/models/SaveTargetRequest";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import type { ImageTemplate } from "../../api/generated/models/ImageTemplate";
import { Button } from "../../components/Button";
import { Field, Select, TextArea, TextInput } from "../../components/form";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { applicationSourcesApi, deploymentTargetsApi } from "../applications/api";
import { applicationEnvsApi } from "../application-envs/api";
import { imageTemplateLabel, imageTemplateOption, imageTemplateOptions, isSafeImageReference } from "./imageTemplates";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";

interface TargetDraft {
  nodeId: string;
  executionMode: string;
  scriptPath: string;
  template: ImageTemplate;
  image: string;
  hostPort: string;
  envFiles: string[];
  parameterSchema: string;
  timeoutSeconds: string;
  verificationConfig: string;
  secretReferences: string;
  privilegedRelease: boolean;
  privilegedReleaseConfirmed: boolean;
}

const initialDraft: TargetDraft = {
  nodeId: "", executionMode: "script", scriptPath: "/srv/apps/example/deploy.sh",
  template: "redis", image: "docker.io/library/redis:7-alpine", hostPort: "6379", envFiles: [],
  parameterSchema: JSON.stringify({ type: "object", properties: {}, additionalProperties: false }, null, 2),
  timeoutSeconds: "900",
  verificationConfig: JSON.stringify({ type: "http", path: "/healthz", expected_status: 200, timeout_ms: 5000 }, null, 2),
  secretReferences: "",
  privilegedRelease: false,
  privilegedReleaseConfirmed: false,
};

function fromTarget(target: DeploymentTargetResponse): TargetDraft {
  const spec = target.imageSpec;
  return {
    nodeId: target.nodeId,
    executionMode: target.executionMode,
    scriptPath: target.scriptPath,
    template: spec?.template ?? "redis",
    image: spec?.image ?? "docker.io/library/redis:7-alpine",
    hostPort: String(spec?.hostPort ?? 6379),
    envFiles: spec?.envFiles ?? [],
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
  const envFiles = useQuery({ queryKey: ["application-env-files", applicationId], queryFn: () => applicationEnvsApi.applicationEnvsList({ applicationId }), enabled: draft.executionMode === "image" });
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
  const selectedNode = nodes.find((node) => node.id === draft.nodeId);
  return <form className="target-form" onSubmit={(event) => void submit(event)}>
    <section className="target-form__panel">
      <div className="target-form__panel-head"><h4>基础配置</h4><p>选择目标节点与执行模式；两阶段模式要求应用已配置并固定 Git 来源。</p></div>
      <div className="target-form__grid">
        <Field label="节点"><Select required value={draft.nodeId} onChange={(event) => setDraft({
          ...draft,
          nodeId: event.target.value,
          privilegedReleaseConfirmed: Boolean(event.target.value === target?.nodeId && target?.privilegedRelease),
        })}><option value="">选择已在线节点</option>{nodes.filter((node) => node.status === "online" || node.id === draft.nodeId).map((node) => <option key={node.id} value={node.id}>{node.name} · {node.host}</option>)}</Select>{hasMoreNodes ? <Button type="button" disabled={loadingMoreNodes} onClick={onLoadMoreNodes}>{loadingMoreNodes ? "正在加载..." : "加载更多节点"}</Button> : null}</Field>
        <Field label="执行模式"><Select required value={draft.executionMode} onChange={(event) => {
          const twoStage = event.target.value === "two_stage";
          const image = event.target.value === "image";
          setDraft({
            ...draft,
            executionMode: event.target.value,
            secretReferences: twoStage || image ? "" : draft.secretReferences,
            privilegedRelease: image ? true : twoStage && draft.privilegedRelease,
            privilegedReleaseConfirmed: image ? false : twoStage && draft.privilegedReleaseConfirmed,
          });
        }}><option value="script">单脚本模式</option><option value="two_stage">两阶段模式（prepare + release）</option><option value="image">镜像直连模式（模板 + 官方镜像）</option></Select></Field>
        {draft.executionMode !== "image" ? <Field label={draft.executionMode === "two_stage" ? "发布脚本路径（Agent 固定执行 make deploy-go-release 的占位路径）" : "脚本绝对路径"} className="form-span"><TextInput required value={draft.scriptPath} onChange={(event) => setDraft({ ...draft, scriptPath: event.target.value })} /></Field> : null}
      </div>
    </section>
    {draft.executionMode === "image" ? <section className="target-form__panel">
      <div className="target-form__panel-head"><h4>镜像部署</h4><p>固定模板、镜像、宿主端口与已登记 Env 文件；不接受任意 Compose、命令、参数或环境变量表。</p></div>
      <div className="target-form__grid">
        <Field label="模板"><Select required value={draft.template} onChange={(event) => {
          const next = imageTemplateOption(event.target.value as ImageTemplate);
          setDraft({ ...draft, template: next.value, image: next.image, hostPort: next.hostPort, envFiles: [], privilegedReleaseConfirmed: false });
        }}>{imageTemplateOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</Select></Field>
        <Field label="镜像引用"><TextInput required value={draft.image} onChange={(event) => setDraft({ ...draft, image: event.target.value, privilegedReleaseConfirmed: false })} /></Field>
        <Field label="宿主端口"><TextInput required type="number" min="1" max="65535" value={draft.hostPort} onChange={(event) => setDraft({ ...draft, hostPort: event.target.value, privilegedReleaseConfirmed: false })} /></Field>
        <div className="form-span">
          <span className="form-label">Env 文件（已登记配置）</span>
          {envFiles.isLoading ? <small>正在加载 Env 文件...</small> : envFiles.isError ? <small className="notice notice--danger">Env 文件列表加载失败</small> : envFiles.data?.items.length === 0 ? <p className="notice">应用尚未登记 Env；请先在应用配置登记，再回来选择镜像部署使用的文件。</p> : <div className="env-file-checkboxes">{envFiles.data?.items.map((file) => <label className="checkbox-field" key={file.id}><input type="checkbox" checked={draft.envFiles.includes(file.fileName)} onChange={(event) => setDraft({ ...draft, envFiles: event.target.checked ? [...draft.envFiles, file.fileName] : draft.envFiles.filter((name) => name !== file.fileName), privilegedReleaseConfirmed: false })} /><span><strong>{file.fileName}</strong><small>{file.module} · v{file.currentVersion}</small></span></label>)}</div>}
        </div>
      </div>
    </section> : null}
    {draft.executionMode === "two_stage" || draft.executionMode === "image" ? <section className="target-form__panel target-form__panel--privilege target-privileged-release">
      <div className="target-form__panel-head"><h4>Agent 原生特权 release</h4><p>release 由目标节点 root executor 执行固定 Make target，prepare 仍由低权限 runner 执行。该开关按部署目标开启：root 发布能力同时绑定应用仓库、固定分支与目标节点，因此需要逐个目标确认。</p></div>
      <div className="target-form__grid">
        <label className="checkbox-field form-span">
          <input type="checkbox" checked={draft.privilegedRelease} disabled={draft.executionMode === "image"} onChange={(event) => setDraft({ ...draft, privilegedRelease: event.target.checked, privilegedReleaseConfirmed: false })} />
          <span><strong>使用 Agent 原生特权 release</strong><small>{draft.executionMode === "image" ? "镜像直连部署必须开启特权 release，由 root executor 执行固定 Make target。" : "release 将由目标节点 root executor 执行固定 Make target；prepare 仍使用低权限 runner。"}</small></span>
        </label>
        {draft.privilegedRelease ? <div className="target-privileged-release__confirmation form-span">
          {draft.executionMode === "image" ? <dl className="definition-grid target-privileged-release__summary">
            <div><dt>模板</dt><dd><code>{imageTemplateLabel(draft.template)}</code></dd></div>
            <div><dt>镜像</dt><dd><code>{draft.image || "尚未填写"}</code></dd></div>
            <div><dt>宿主端口</dt><dd><code>{draft.hostPort || "尚未填写"}</code></dd></div>
            <div><dt>目标节点</dt><dd>{selectedNode ? <span className="target-node-summary"><code>{selectedNode.name}</code><span className={`status-badge status-badge--${selectedNode.status === "online" ? "online" : "offline"}`}>{selectedNode.status === "online" ? "在线" : "离线"}</span></span> : <code>{draft.nodeId || "尚未选择"}</code>}</dd></div>
          </dl> : <dl className="definition-grid target-privileged-release__summary">
            <div><dt>仓库</dt><dd><code>{source.data?.repositoryUrl ?? "来源尚未加载"}</code></dd></div>
            <div><dt>固定分支</dt><dd><code>{source.data?.deploymentBranch ?? "尚未固定"}</code></dd></div>
            <div><dt>目标节点</dt><dd>{selectedNode ? <span className="target-node-summary"><code>{selectedNode.name}</code><span className={`status-badge status-badge--${selectedNode.status === "online" ? "online" : "offline"}`}>{selectedNode.status === "online" ? "在线" : "离线"}</span></span> : <code>{draft.nodeId || "尚未选择"}</code>}</dd></div>
          </dl>}
          <label className="checkbox-field checkbox-field--danger">
            <input type="checkbox" checked={draft.privilegedReleaseConfirmed} onChange={(event) => setDraft({ ...draft, privilegedReleaseConfirmed: event.target.checked })} />
            <span>{draft.executionMode === "image" ? "我确认该镜像、模板与宿主端口将由目标节点 root executor 固定执行平台生成的 Make target" : "我确认该仓库和固定分支的写入者将获得目标节点 root 发布能力"}</span>
          </label>
        </div> : null}
      </div>
    </section> : null}
    <section className="target-form__panel">
      <div className="target-form__panel-head"><h4>执行与验证</h4><p>超时、参数 Schema 与部署后验证配置；旧版单脚本模式的敏感文件引用按需显示。</p></div>
      <div className="target-form__grid">
        <Field label="超时秒数"><TextInput required type="number" min="1" max="86400" value={draft.timeoutSeconds} onChange={(event) => setDraft({ ...draft, timeoutSeconds: event.target.value })} /></Field>
        {draft.executionMode === "script" ? <Field label="敏感文件引用（旧版单脚本模式）" hint={"仅单脚本模式使用；两阶段 release 从应用配置读取 Env，无需在此配置。"}><TextArea rows={4} value={draft.secretReferences} onChange={(event) => setDraft({ ...draft, secretReferences: event.target.value })} placeholder={"DEPLOY_TOKEN_FILE=/srv/secrets/app/token\nENV_FILE=/srv/secrets/app/.env"} /></Field> : null}
        {draft.executionMode !== "image" ? <Field label="参数 JSON Schema" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.parameterSchema} onChange={(event) => setDraft({ ...draft, parameterSchema: event.target.value })} /></Field> : null}
        {draft.executionMode !== "image" ? <Field label="部署后验证配置" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.verificationConfig} onChange={(event) => setDraft({ ...draft, verificationConfig: event.target.value })} /></Field> : null}
      </div>
    </section>
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
  if (draft.executionMode === "image") {
    if (!draft.privilegedRelease) throw new Error("镜像执行模式必须开启 Agent 原生特权 release");
    if (!draft.privilegedReleaseConfirmed) throw new Error("开启 Agent 原生特权 release 前必须确认 root 信任边界");
    const hostPort = Number(draft.hostPort);
    if (!Number.isInteger(hostPort) || hostPort < 1 || hostPort > 65535) throw new Error("宿主端口必须在 1-65535 之间");
    if (!isSafeImageReference(draft.image)) throw new Error("镜像引用只允许安全字符，不能以连字符或 URL scheme 开头");
    if (draft.envFiles.length === 0 || draft.envFiles.length > 16) throw new Error("镜像部署必须选择 1-16 个已登记 Env 文件");
    return {
      nodeId: draft.nodeId,
      executionMode: draft.executionMode,
      scriptPath: "",
      parameterSchema: {},
      timeoutSeconds: Number(draft.timeoutSeconds),
      verificationConfig: {},
      secretFileReferences: [],
      privilegedRelease: true,
      privilegedReleaseConfirmed: true,
      imageSpec: { template: draft.template, image: draft.image.trim(), hostPort, envFiles: draft.envFiles },
      version,
    };
  }
  const secretFileReferences = draft.executionMode === "script"
    ? draft.secretReferences.split("\n").map((line) => line.trim()).filter(Boolean).map((line) => {
      const separator = line.indexOf("=");
      if (separator <= 0 || separator === line.length - 1) throw new Error("敏感文件引用必须使用 ENV_KEY=/absolute/path 格式");
      return { environmentKey: line.slice(0, separator).trim(), filePath: line.slice(separator + 1).trim() };
    })
    : [];
  if (draft.privilegedRelease && !draft.privilegedReleaseConfirmed) throw new Error("开启 Agent 原生特权 release 前必须确认 root 信任边界");
  return { nodeId: draft.nodeId, executionMode: draft.executionMode, scriptPath: draft.scriptPath.trim(), parameterSchema, timeoutSeconds: Number(draft.timeoutSeconds), verificationConfig, secretFileReferences, privilegedRelease: draft.privilegedRelease, privilegedReleaseConfirmed: draft.privilegedReleaseConfirmed, version };
}

function isObject(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
