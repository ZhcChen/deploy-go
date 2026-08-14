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
import { deploymentTargetsApi } from "../applications/api";
import { applicationEnvsApi } from "../application-envs/api";
import { hasRequiredImageEnvFiles, imageTemplateOption, imageTemplateOptions, imageTemplateRequiredEnvFiles, isSafeImageReference } from "./imageTemplates";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";

interface TargetDraft {
  nodeId: string;
  targetCode: string;
  executionMode: string;
  scriptPath: string;
  template: ImageTemplate;
  image: string;
  hostPort: string;
  envFiles: string[];
  timeoutSeconds: string;
  secretReferences: string;
}

const initialDraft: TargetDraft = {
  nodeId: "", targetCode: "", executionMode: "script", scriptPath: "/srv/apps/example/deploy.sh",
  template: "redis", image: "docker.io/library/redis:7-alpine", hostPort: "6379", envFiles: [],
  timeoutSeconds: "900",
  secretReferences: "",
};

function fromTarget(target: DeploymentTargetResponse): TargetDraft {
  const spec = target.imageSpec;
  return {
    nodeId: target.nodeId,
    targetCode: target.targetCode,
    executionMode: target.executionMode,
    scriptPath: target.scriptPath,
    template: spec?.template ?? "redis",
    image: spec?.image ?? "docker.io/library/redis:7-alpine",
    hostPort: String(spec?.hostPort ?? 6379),
    envFiles: spec?.envFiles ?? [],
    timeoutSeconds: String(target.timeoutSeconds),
    secretReferences: target.secretFileReferences.map((item) => `${item.environmentKey}=${item.filePath}`).join("\n"),
  };
}

export function TargetEditor({ applicationId, nodes, target, hasMoreNodes, loadingMoreNodes, onLoadMoreNodes, onDiscard, onSaved }: { applicationId: string; nodes: NodeResponse[]; target?: DeploymentTargetResponse; hasMoreNodes?: boolean; loadingMoreNodes?: boolean; onLoadMoreNodes?(): void; onDiscard(): void; onSaved?(target: DeploymentTargetResponse): void }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState(() => target ? fromTarget(target) : initialDraft);
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
  return <form className="target-form" onSubmit={(event) => void submit(event)}>
    <section className="target-form__panel">
      <div className="target-form__panel-head"><h4>基础配置</h4><p>选择目标节点与执行模式；两阶段模式要求应用已配置并固定 Git 来源。</p></div>
      <div className="target-form__grid">
        <Field label="节点"><Select required value={draft.nodeId} onChange={(event) => setDraft({
          ...draft,
          nodeId: event.target.value,
        })}><option value="">选择已在线节点</option>{nodes.filter((node) => node.status === "online" || node.id === draft.nodeId).map((node) => <option key={node.id} value={node.id}>{node.name} · {node.host}</option>)}</Select>{hasMoreNodes ? <Button type="button" disabled={loadingMoreNodes} onClick={onLoadMoreNodes}>{loadingMoreNodes ? "正在加载..." : "加载更多节点"}</Button> : null}</Field>
        <Field label="目标稳定标识（target_code）" hint="executor 用它定位本机 Compose 项目；留空时按环境标识生成，绑定已有容器时填现有项目名，例如 shared-prod-redis。"><TextInput value={draft.targetCode} onChange={(event) => setDraft({ ...draft, targetCode: event.target.value })} /></Field>
        <Field label="执行模式"><Select required value={draft.executionMode} onChange={(event) => {
          const twoStage = event.target.value === "two_stage";
          const image = event.target.value === "image";
          setDraft({
            ...draft,
            executionMode: event.target.value,
            secretReferences: twoStage || image ? "" : draft.secretReferences,
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
          setDraft({ ...draft, template: next.value, image: next.image, hostPort: next.hostPort, envFiles: [] });
        }}>{imageTemplateOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</Select></Field>
        <Field label="镜像引用"><TextInput required value={draft.image} onChange={(event) => setDraft({ ...draft, image: event.target.value })} /></Field>
        <Field label="宿主端口"><TextInput required type="number" min="1" max="65535" value={draft.hostPort} onChange={(event) => setDraft({ ...draft, hostPort: event.target.value })} /></Field>
        <div className="form-span">
          <span className="form-label">Env 文件（已登记配置）</span>
          {envFiles.isLoading ? <small>正在加载 Env 文件...</small> : envFiles.isError ? <small className="notice notice--danger">Env 文件列表加载失败</small> : envFiles.data?.items.length === 0 ? <p className="notice">应用尚未登记 Env；请先在应用配置登记，再回来选择镜像部署使用的文件。</p> : <div className="env-file-checkboxes">{envFiles.data?.items.map((file) => <label className="checkbox-field" key={file.id}><input type="checkbox" checked={draft.envFiles.includes(file.fileName)} onChange={(event) => setDraft({ ...draft, envFiles: event.target.checked ? [...draft.envFiles, file.fileName] : draft.envFiles.filter((name) => name !== file.fileName) })} /><span><strong>{file.fileName}</strong><small>{file.module} · v{file.currentVersion}{imageTemplateRequiredEnvFiles(draft.template).includes(file.fileName) ? " · 模板必选" : ""}</small></span></label>)}</div>}
        </div>
      </div>
    </section> : null}
    <section className="target-form__panel">
      <div className="target-form__panel-head"><h4>执行与验证</h4><p>超时按目标配置；参数 Schema 与部署后验证配置在应用详情统一维护，目标读取应用级生效值。</p></div>
      <div className="target-form__grid">
        <Field label="超时秒数"><TextInput required type="number" min="1" max="86400" value={draft.timeoutSeconds} onChange={(event) => setDraft({ ...draft, timeoutSeconds: event.target.value })} /></Field>
        {draft.executionMode === "script" ? <Field label="敏感文件引用（旧版单脚本模式）" hint={"仅单脚本模式使用；两阶段 release 从应用配置读取 Env，无需在此配置。"}><TextArea rows={4} value={draft.secretReferences} onChange={(event) => setDraft({ ...draft, secretReferences: event.target.value })} placeholder={"DEPLOY_TOKEN_FILE=/srv/secrets/app/token\nENV_FILE=/srv/secrets/app/.env"} /></Field> : null}
      </div>
    </section>
    {parseError ? <div className="notice notice--danger form-span" role="alert"><strong>{parseError}</strong></div> : null}
    {save.error ? <div className="form-span"><ApiErrorNotice error={toNotice(save.error)} /></div> : null}
    <div className="form-actions form-span"><Button type="button" onClick={onDiscard}>丢弃草稿</Button><Button tone="primary" disabled={save.isPending}>{save.isPending ? "正在保存..." : "保存目标"}</Button></div>
  </form>;
}

function parseDraft(draft: TargetDraft, version?: number): SaveTargetRequest {
  if (draft.executionMode === "image") {
    const hostPort = Number(draft.hostPort);
    if (!Number.isInteger(hostPort) || hostPort < 1 || hostPort > 65535) throw new Error("宿主端口必须在 1-65535 之间");
    if (!isSafeImageReference(draft.image)) throw new Error("镜像引用只允许安全字符，不能以连字符或 URL scheme 开头");
    if (draft.envFiles.length === 0 || draft.envFiles.length > 16) throw new Error("镜像部署必须选择 1-16 个已登记 Env 文件");
    if (!hasRequiredImageEnvFiles(draft.template, draft.envFiles)) throw new Error(`镜像部署必须包含模板必选 Env 文件：${imageTemplateRequiredEnvFiles(draft.template).join("、")}`);
    return {
      nodeId: draft.nodeId,
      targetCode: draft.targetCode.trim() || undefined,
      executionMode: draft.executionMode,
      scriptPath: "",
      timeoutSeconds: Number(draft.timeoutSeconds),
      secretFileReferences: [],
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
  return { nodeId: draft.nodeId, targetCode: draft.targetCode.trim() || undefined, executionMode: draft.executionMode, scriptPath: draft.scriptPath.trim(), timeoutSeconds: Number(draft.timeoutSeconds), secretFileReferences, version };
}
