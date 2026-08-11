import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, FileDown, GitBranch, Layers } from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from "react";
import { Link, useSearchParams } from "react-router-dom";
import type { ApplicationResponse } from "../../api/generated/models/ApplicationResponse";
import type { ApplicationSourceResponse } from "../../api/generated/models/ApplicationSourceResponse";
import type { DeploymentTargetResponse } from "../../api/generated/models/DeploymentTargetResponse";
import type { GitRefDiscoveryResponse } from "../../api/generated/models/GitRefDiscoveryResponse";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field, Select, TextArea, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { useAuth } from "../auth/AuthContext";
import { applicationNodesApi, applicationsApi, applicationSourcesApi, deploymentTargetsApi, gitCredentialsApi, sourceAgentsApi } from "../applications/api";
import { applicationTemplates } from "./applicationTemplates";
import type { TemplateFile } from "./applicationTemplates";
import { defaultScriptPath, downloadTemplateFile, findTemplate, slugify, templateDefaults, templateEnvExamples, templateParameterSchema } from "./createFromTemplate";

type Step = "template" | "app" | "source" | "target" | "done";

interface AppDraft {
  name: string;
  slug: string;
  description: string;
}

interface SourceDraft {
  repositoryUrl: string;
  gitCredentialId: string;
  buildAgentId: string;
}

interface TargetDraft {
  nodeId: string;
  scriptPath: string;
  timeoutSeconds: string;
  parameterSchema: string;
  verificationConfig: string;
  privilegedRelease: boolean;
  privilegedReleaseConfirmed: boolean;
}

const discoveryLabels: Record<string, string> = {
  queued: "等待 Agent 查询",
  running: "正在查询远程分支",
  succeeded: "查询完成",
  failed: "查询失败",
  expired: "结果已过期",
};

const discoveryErrorLabels: Record<string, string> = {
  git_authentication_failed: "Git 认证失败，请检查凭证是否已配置为只读 deploy key。",
  git_repository_unreachable: "仓库不可达，请检查地址和网络。",
  git_ref_discovery_timeout: "分支查询超时，请稍后重试。",
  secret_lease_failed: "Git 私钥租约获取失败，请检查凭证状态。",
};

function initialTargetDraft(template: ReturnType<typeof findTemplate>, slug: string, workRoot?: string): TargetDraft {
  const parameterSchema = template ? JSON.stringify(templateParameterSchema(template), null, 2) : "{\n  \"type\": \"object\",\n  \"properties\": {},\n  \"required\": [],\n  \"additionalProperties\": false\n}";
  return {
    nodeId: "",
    scriptPath: defaultScriptPath(workRoot, slug),
    timeoutSeconds: "900",
    parameterSchema,
    verificationConfig: JSON.stringify(template ? templateDefaults(template).verificationConfig : {}, null, 2),
    privilegedRelease: false,
    privilegedReleaseConfirmed: false,
  };
}

function initialAppDraft(template: ReturnType<typeof findTemplate>): AppDraft {
  const defaults = template ? templateDefaults(template) : null;
  return defaults ? {
    name: defaults.appName,
    slug: defaults.slugSuggestion,
    description: defaults.description,
  } : { name: "", slug: "", description: "" };
}

export function CreateFromTemplatePage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const [step, setStep] = useState<Step>("template");
  const [templateId, setTemplateId] = useState(searchParams.get("template") ?? "postgres");
  const template = findTemplate(templateId) ?? applicationTemplates[0];
  const defaults = template ? templateDefaults(template) : null;
  const [appDraft, setAppDraft] = useState<AppDraft>(() => initialAppDraft(template));
  const [createdApp, setCreatedApp] = useState<ApplicationResponse | null>(null);
  const [sourceDraft, setSourceDraft] = useState<SourceDraft>({ repositoryUrl: "", gitCredentialId: "", buildAgentId: "" });
  const [source, setSource] = useState<ApplicationSourceResponse | null>(null);
  const [discovery, setDiscovery] = useState<GitRefDiscoveryResponse | null>(null);
  const [selectedBranch, setSelectedBranch] = useState("");
  const [targetDraft, setTargetDraft] = useState<TargetDraft>(() => initialTargetDraft(template, defaults?.slugSuggestion ?? ""));
  const [createdTarget, setCreatedTarget] = useState<DeploymentTargetResponse | null>(null);

  function updateSourceDraft(patch: Partial<SourceDraft>) {
    setSourceDraft((current) => ({ ...current, ...patch }));
    setSource(null);
    setDiscovery(null);
    setSelectedBranch("");
  }

  const credentials = useQuery({ queryKey: ["git-credentials", "wizard"], queryFn: () => gitCredentialsApi.gitCredentialsList({ limit: 200 }), enabled: step === "source" });
  const agents = useQuery({ queryKey: ["agents", "wizard"], queryFn: () => sourceAgentsApi.agentsList({ limit: 200 }), enabled: step === "source" });
  const nodes = useQuery({ queryKey: ["nodes", "wizard"], queryFn: () => applicationNodesApi.nodesList({ limit: 200 }), enabled: step === "target" });
  const usableAgents = (agents.data?.items ?? []).filter((agent) => agent.status === "online" && (agent.protocolVersion ?? 0) >= 2);
  const usableCredentials = (credentials.data?.items ?? []).filter((credential) => credential.status === "active");
  const onlineNodes = (nodes.data?.items ?? []).filter((node) => node.status === "online");

  const createApp = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return applicationsApi.applicationsCreate({ xCSRFToken: auth.csrfToken, saveApplicationRequest: { ...appDraft, name: appDraft.name.trim(), slug: appDraft.slug.trim(), description: appDraft.description.trim() } });
    },
    onSuccess: async (saved) => {
      setCreatedApp(saved);
      await queryClient.invalidateQueries({ queryKey: ["applications"] });
      setStep("source");
    },
  });

  const saveSource = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !createdApp) throw new Error("缺少必要的安全上下文");
      const saved = await applicationSourcesApi.applicationSourceSave({
        applicationId: createdApp.id,
        xCSRFToken: auth.csrfToken,
        saveSourceRequest: {
          repositoryUrl: sourceDraft.repositoryUrl.trim(),
          gitCredentialId: sourceDraft.gitCredentialId || null,
          buildAgentId: sourceDraft.buildAgentId,
          sourcePolicy: "branch",
        },
      });
      setSource(saved);
      setDiscovery(null);
      setSelectedBranch("");
      const started = await applicationSourcesApi.applicationSourceRefreshRefs({ applicationId: createdApp.id, xCSRFToken: auth.csrfToken });
      setDiscovery(started);
      return saved;
    },
  });

  const refreshDiscovery = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !createdApp) throw new Error("缺少必要的安全上下文");
      const started = await applicationSourcesApi.applicationSourceRefreshRefs({ applicationId: createdApp.id, xCSRFToken: auth.csrfToken });
      setDiscovery(started);
      setSelectedBranch("");
      return started;
    },
  });

  const fixBranch = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !createdApp || !source) throw new Error("缺少必要的安全上下文");
      return applicationSourcesApi.applicationSourceSetBranch({
        applicationId: createdApp.id,
        xCSRFToken: auth.csrfToken,
        setBranchRequest: { branch: selectedBranch, version: source.version },
      });
    },
    onSuccess: (saved) => {
      setSource(saved);
      setDiscovery(null);
      setStep("target");
    },
  });

  const createTarget = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !createdApp) throw new Error("缺少必要的安全上下文");
      if (targetDraft.privilegedRelease && !targetDraft.privilegedReleaseConfirmed) {
        throw new Error("开启 Agent 原生特权 release 前必须确认 root 信任边界");
      }
      let parameterSchema: unknown;
      let verificationConfig: unknown;
      try {
        parameterSchema = JSON.parse(targetDraft.parameterSchema) as unknown;
        verificationConfig = JSON.parse(targetDraft.verificationConfig) as unknown;
      } catch {
        throw new Error("参数 Schema 或验证配置不是有效 JSON");
      }
      if (typeof parameterSchema !== "object" || parameterSchema === null || Array.isArray(parameterSchema)) throw new Error("参数 Schema 必须是 JSON object");
      if (typeof verificationConfig !== "object" || verificationConfig === null || Array.isArray(verificationConfig)) throw new Error("验证配置必须是 JSON object");
      return deploymentTargetsApi.deploymentTargetsCreate({
        applicationId: createdApp.id,
        xCSRFToken: auth.csrfToken,
        saveTargetRequest: {
          nodeId: targetDraft.nodeId,
          executionMode: "two_stage",
          scriptPath: targetDraft.scriptPath.trim(),
          parameterSchema,
          timeoutSeconds: Number(targetDraft.timeoutSeconds),
          verificationConfig,
          privilegedRelease: targetDraft.privilegedRelease,
          privilegedReleaseConfirmed: targetDraft.privilegedReleaseConfirmed,
        },
      });
    },
    onSuccess: async (saved) => {
      setCreatedTarget(saved);
      if (createdApp) await queryClient.invalidateQueries({ queryKey: ["deployment-targets", createdApp.id] });
      setStep("done");
    },
  });

  useEffect(() => {
    if (!discovery || !createdApp || !["queued", "running"].includes(discovery.status)) return;
    const timer = window.setTimeout(() => {
      void applicationSourcesApi.applicationSourceRefreshShow({ applicationId: createdApp.id, refsQueryId: discovery.id })
        .then(setDiscovery)
        .catch(() => setDiscovery((current) => current ? { ...current, status: "failed", errorCode: "git_ref_discovery_failed" } : current));
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [discovery, createdApp]);

  const envExamples = useMemo(() => template ? templateEnvExamples(template) : null, [template]);

  async function submitApp(event: FormEvent) {
    event.preventDefault();
    await createApp.mutateAsync().catch(() => undefined);
  }

  async function submitSource(event: FormEvent) {
    event.preventDefault();
    await saveSource.mutateAsync().catch(() => undefined);
  }

  async function submitTarget(event: FormEvent) {
    event.preventDefault();
    await createTarget.mutateAsync().catch(() => undefined);
  }

  function changeTemplate(nextTemplateId: string) {
    setTemplateId(nextTemplateId);
    const nextTemplate = findTemplate(nextTemplateId);
    if (!nextTemplate) return;
    const defaults = templateDefaults(nextTemplate);
    setAppDraft({ name: defaults.appName, slug: defaults.slugSuggestion, description: defaults.description });
    setTargetDraft(initialTargetDraft(nextTemplate, defaults.slugSuggestion));
  }

  if (!template || !defaults) return <section className="workspace"><PageState kind="empty" /></section>;

  return <section className="workspace template-wizard">
    <BackLink to="/templates" parentLabel="应用模板" />
    <div className="workspace-heading"><div><h2>从模板创建应用</h2><p>按模板预填应用、Git 来源与两阶段部署目标；创建结果会引导你准备 Env 与业务仓库。</p></div></div>
    <WizardSteps current={step} />
    {step === "template" ? <TemplateStep template={template} templates={applicationTemplates} onChoose={changeTemplate} onNext={() => setStep("app")} onDownload={(file) => downloadTemplateFile(template, file)} /> : null}
    {step === "app" ? <section className="wizard-panel">
      <div className="wizard-panel__head"><h3>应用信息</h3><p>名称与 Slug 会进入部署目标和业务目录命名。</p></div>
      <form className="node-form" onSubmit={(event) => void submitApp(event)}>
        <Field label="应用名称"><TextInput required maxLength={100} value={appDraft.name} onChange={(event) => setAppDraft((current) => ({ ...current, name: event.target.value, slug: current.slug === defaults.slugSuggestion ? slugify(event.target.value, defaults.slugSuggestion) : current.slug }))} /></Field>
        <Field label="Slug"><TextInput required pattern="[a-z0-9][a-z0-9-]*" value={appDraft.slug} onChange={(event) => setAppDraft({ ...appDraft, slug: event.target.value })} /></Field>
        <Field label="说明" className="form-span"><TextArea rows={3} value={appDraft.description} onChange={(event) => setAppDraft({ ...appDraft, description: event.target.value })} /></Field>
        {createApp.error ? <div className="form-span"><ApiErrorNotice error={toNotice(createApp.error)} /></div> : null}
        <div className="form-actions form-span"><Button type="button" onClick={() => setStep("template")}>上一步</Button><Button tone="primary" disabled={createApp.isPending}>{createApp.isPending ? "正在创建应用..." : "创建应用并继续"}</Button></div>
      </form>
    </section> : null}
    {step === "source" ? <SourceStep
      createdApp={createdApp}
      sourceDraft={sourceDraft}
      setSourceDraft={updateSourceDraft}
      source={source}
      discovery={discovery}
      selectedBranch={selectedBranch}
      setSelectedBranch={setSelectedBranch}
      credentials={usableCredentials}
      agents={usableAgents}
      loadingOptions={credentials.isLoading || agents.isLoading}
      saving={saveSource.isPending || refreshDiscovery.isPending}
      fixing={fixBranch.isPending}
      error={saveSource.error ?? refreshDiscovery.error ?? fixBranch.error}
      onSubmit={submitSource}
      onRefresh={() => void refreshDiscovery.mutateAsync().catch(() => undefined)}
      onFix={() => void fixBranch.mutateAsync().catch(() => undefined)}
      onSkip={() => setStep("done")}
      onBack={() => setStep("app")}
      canGoBack={!createdApp}
      envExamples={envExamples}
      template={template}
      appLink={createdApp ? <Link className="text-link" to={`/apps/${createdApp.id}`}>{createdApp.name}</Link> : null}
    /> : null}
    {step === "target" ? <TargetStep
      draft={targetDraft}
      setDraft={(patch) => setTargetDraft((current) => ({ ...current, ...patch }))}
      nodes={onlineNodes}
      source={source}
      appSlug={createdApp?.slug ?? appDraft.slug}
      error={createTarget.error}
      pending={createTarget.isPending}
      onSubmit={submitTarget}
      onBack={() => setStep("source")}
      onSkip={() => setStep("done")}
      appLink={createdApp ? <Link className="text-link" to={`/apps/${createdApp.id}`}>{createdApp.name}</Link> : null}
    /> : null}
    {step === "done" ? <DoneStep
      app={createdApp}
      source={source}
      target={createdTarget}
      envExamples={envExamples}
      onRestart={() => {
        setStep("template");
        setAppDraft(initialAppDraft(template));
        setSourceDraft({ repositoryUrl: "", gitCredentialId: "", buildAgentId: "" });
        setCreatedApp(null);
        setSource(null);
        setDiscovery(null);
        setSelectedBranch("");
        setCreatedTarget(null);
        setTargetDraft(initialTargetDraft(template, defaults.slugSuggestion));
      }}
    /> : null}
  </section>;
}

function WizardSteps({ current }: { current: Step }) {
  const steps: Array<{ id: Step; label: string }> = [
    { id: "template", label: "选择模板" },
    { id: "app", label: "应用信息" },
    { id: "source", label: "Git 来源" },
    { id: "target", label: "部署目标" },
    { id: "done", label: "完成" },
  ];
  const active = steps.findIndex((item) => item.id === current);
  return <ol className="wizard-steps" aria-label="创建向导步骤">{steps.map((item, index) => <li key={item.id} className={index <= active ? "is-active" : ""} aria-current={item.id === current ? "step" : undefined}><span>{index + 1}</span>{item.label}</li>)}</ol>;
}

function TemplateStep({ template, templates, onChoose, onNext, onDownload }: { template: ReturnType<typeof findTemplate>; templates: typeof applicationTemplates; onChoose(id: string): void; onNext(): void; onDownload(file: TemplateFile): void }) {
  if (!template) return null;
  return <section className="wizard-panel">
    <div className="wizard-panel__head"><h3>选择应用模板</h3><p>模板只读提供 Compose、Env 示例与应用配置；部署前仍需复制到独立业务仓库。</p></div>
    <div className="template-selector" role="tablist" aria-label="选择应用模板">
      {templates.map((item) => <button key={item.id} type="button" role="tab" aria-selected={template.id === item.id} onClick={() => onChoose(item.id)}><Layers aria-hidden="true" /><span><strong>{item.name}</strong><small>{item.summary}</small></span></button>)}
    </div>
    <div className="template-wizard-files">
      <div className="section-heading"><div><h4>模板文件</h4><p>Env 示例与配置文件可复制或下载到独立仓库。</p></div></div>
      <ul>{template.files.map((file) => <li key={file.path}><code>{file.path}</code><Button type="button" onClick={() => onDownload(file)}><FileDown aria-hidden="true" />下载</Button></li>)}</ul>
    </div>
    <div className="form-actions"><Button type="button" tone="primary" onClick={onNext}>使用 {template.name} 继续</Button></div>
  </section>;
}

function SourceStep({ createdApp, sourceDraft, setSourceDraft, source, discovery, selectedBranch, setSelectedBranch, credentials, agents, loadingOptions, saving, fixing, error, onSubmit, onRefresh, onFix, onSkip, onBack, canGoBack, envExamples, template, appLink }: {
  createdApp: ApplicationResponse | null;
  sourceDraft: SourceDraft;
  setSourceDraft(patch: Partial<SourceDraft>): void;
  source: ApplicationSourceResponse | null;
  discovery: GitRefDiscoveryResponse | null;
  selectedBranch: string;
  setSelectedBranch(value: string): void;
  credentials: Array<{ id: string; name: string }>;
  agents: Array<{ id: string; name: string; agentVersion?: string | null }>;
  loadingOptions: boolean;
  saving: boolean;
  fixing: boolean;
  error: unknown;
  onSubmit(event: FormEvent): void;
  onRefresh(): void;
  onFix(): void;
  onSkip(): void;
  onBack(): void;
  canGoBack: boolean;
  envExamples: ReturnType<typeof templateEnvExamples> | null;
  template: NonNullable<ReturnType<typeof findTemplate>>;
  appLink: ReactNode;
}) {
  const discoveryPending = Boolean(discovery && ["queued", "running"].includes(discovery.status));
  const discoveryFailed = Boolean(discovery && (discovery.status === "failed" || discovery.status === "expired"));
  return <section className="wizard-panel">
    <div className="wizard-panel__head"><h3>Git 来源与固定分支</h3><p>两阶段目标要求先保存来源并固定部署分支；若仓库尚未推送模板，可先“仅创建应用”。</p></div>
    {source ? <dl className="definition-grid"><div><dt>仓库地址</dt><dd><code>{source.repositoryUrl}</code></dd></div><div><dt>来源状态</dt><dd>{source.status === "verified" ? "已验证" : "草稿"}</dd></div></dl> : null}
    <form className="source-form" onSubmit={(event) => void onSubmit(event)}>
      <Field label="仓库地址" className="form-span"><TextInput required value={sourceDraft.repositoryUrl} onChange={(event) => setSourceDraft({ repositoryUrl: event.target.value })} placeholder="git@github.com:org/my-postgres.git" /></Field>
      <Field label="Git 凭证"><Select required value={sourceDraft.gitCredentialId} onChange={(event) => setSourceDraft({ gitCredentialId: event.target.value })}><option value="">公开仓库（无凭证）</option>{credentials.map((credential) => <option key={credential.id} value={credential.id}>{credential.name}</option>)}</Select>{loadingOptions ? <small>正在加载凭证...</small> : null}</Field>
      <Field label="构建节点"><Select required value={sourceDraft.buildAgentId} onChange={(event) => setSourceDraft({ buildAgentId: event.target.value })}><option value="">选择在线节点</option>{agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name} · v{agent.agentVersion || "-"}</option>)}</Select>{loadingOptions ? <small>正在加载节点...</small> : null}</Field>
      <div className="form-actions form-span">{canGoBack ? <Button type="button" onClick={onBack}>上一步</Button> : null}<Button tone="primary" disabled={saving || fixing || !createdApp}>{saving ? "正在保存并扫描..." : "保存来源并扫描分支"}</Button><Button type="button" onClick={onSkip}>仅创建应用</Button></div>
      {error ? <div className="form-span"><ApiErrorNotice error={toNotice(error)} /></div> : null}
      {createdApp && error ? <p className="notice form-span">应用已创建：{appLink}；来源配置失败不会回滚，可修正后重试或仅创建应用。</p> : null}
    </form>
    {discoveryPending ? <div className="source-discovery"><div className="section-heading"><div><h4>分支发现</h4><p>{discoveryLabels[discovery?.status ?? ""] ?? "查询中"}</p></div></div><PageState kind="loading" /></div> : null}
    {discovery?.status === "succeeded" ? <div className="source-discovery">
      <div className="section-heading"><div><h4>分支发现</h4><p>已发现 {discovery.refs.length} 个分支，选择固定部署分支。</p></div></div>
      {discovery.refs.length === 0 ? <p className="notice">远程仓库没有可部署分支。</p> : <div className="branch-fix-form"><Field label="固定分支"><Select required value={selectedBranch} onChange={(event) => setSelectedBranch(event.target.value)}><option value="">选择分支</option>{discovery.refs.map((ref) => <option key={ref.name} value={ref.name}>{ref.name} · {ref.sha.slice(0, 10)}</option>)}</Select></Field><div className="form-actions"><Button type="button" tone="primary" disabled={fixing || !selectedBranch || !source} onClick={onFix}><GitBranch aria-hidden="true" />{fixing ? "正在固定..." : "固定分支并继续"}</Button></div></div>}
    </div> : null}
    {discoveryFailed ? <div className="source-discovery"><div className="section-heading"><div><h4>分支发现</h4><p className="notice notice--danger" role="alert">{discoveryErrorLabels[discovery?.errorCode ?? ""] ?? "分支查询失败，请重新扫描。"}</p></div></div><div className="form-actions"><Button type="button" disabled={saving} onClick={onRefresh}>重新扫描</Button></div></div> : null}
    {envExamples && createdApp ? <section className="wizard-env-examples">
      <div className="section-heading"><div><h4>Env 示例（结果页会再次提供）</h4><p>{template.name} 需要两个 Env 文件；真实值不要写入仓库。</p></div></div>
      <ClipboardFallback value={envExamples.composeEnv} label="复制 compose.env 示例" />
      <ClipboardFallback value={envExamples.serviceEnv} label={`复制 ${template.id}.env 示例`} />
    </section> : null}
  </section>;
}

function TargetStep({ draft, setDraft, nodes, source, appSlug, error, pending, onSubmit, onBack, onSkip, appLink }: {
  draft: TargetDraft;
  setDraft(patch: Partial<TargetDraft>): void;
  nodes: NodeResponse[];
  source: ApplicationSourceResponse | null;
  appSlug: string;
  error: unknown;
  pending: boolean;
  onSubmit(event: FormEvent): void;
  onBack(): void;
  onSkip(): void;
  appLink: ReactNode;
}) {
  const selectedNode = nodes.find((node) => node.id === draft.nodeId);
  return <section className="wizard-panel">
    <div className="wizard-panel__head"><h3>部署目标</h3><p>参数 Schema 与验证配置已按模板预填；特权 release 默认关闭。</p></div>
    <form className="target-form" onSubmit={(event) => void onSubmit(event)}>
      <div className="target-form__grid">
        <Field label="节点"><Select required value={draft.nodeId} onChange={(event) => {
          const nextNode = nodes.find((node) => node.id === event.target.value);
          setDraft({ nodeId: event.target.value, scriptPath: nextNode ? defaultScriptPath(nextNode.workRoot, appSlug) : draft.scriptPath, privilegedReleaseConfirmed: false });
        }}><option value="">选择已在线节点</option>{nodes.map((node) => <option key={node.id} value={node.id}>{node.name} · {node.host}</option>)}</Select></Field>
        <Field label="执行模式"><TextInput readOnly value="两阶段模式（prepare + release）" /></Field>
        <Field label="发布脚本路径（占位）" hint="实际由 root executor 固定执行 make deploy-go-release。" className="form-span"><TextInput required value={draft.scriptPath} onChange={(event) => setDraft({ scriptPath: event.target.value })} /></Field>
        <Field label="超时秒数"><TextInput required type="number" min="1" max="86400" value={draft.timeoutSeconds} onChange={(event) => setDraft({ timeoutSeconds: event.target.value })} /></Field>
        <Field label="参数 JSON Schema" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.parameterSchema} onChange={(event) => setDraft({ parameterSchema: event.target.value })} /></Field>
        <Field label="部署后验证配置" className="form-span"><TextArea rows={12} spellCheck={false} value={draft.verificationConfig} onChange={(event) => setDraft({ verificationConfig: event.target.value })} /></Field>
      </div>
      <section className="target-form__panel target-form__panel--privilege target-privileged-release">
        <div className="target-form__panel-head"><h4>Agent 原生特权 release</h4><p>release 由目标节点 root executor 执行固定 Make target；开启即把 root 发布能力交给该仓库固定分支的写入者。</p></div>
        <label className="checkbox-field">
          <input type="checkbox" checked={draft.privilegedRelease} onChange={(event) => setDraft({ privilegedRelease: event.target.checked, privilegedReleaseConfirmed: false })} />
          <span><strong>使用 Agent 原生特权 release</strong><small>需要节点 Agent 0.2.0、控制协议 v7 与 executor v2。</small></span>
        </label>
        {draft.privilegedRelease ? <label className="checkbox-field checkbox-field--danger">
          <input type="checkbox" checked={draft.privilegedReleaseConfirmed} onChange={(event) => setDraft({ privilegedReleaseConfirmed: event.target.checked })} />
          <span>我确认该仓库和固定分支的写入者将获得目标节点 root 发布能力</span>
        </label> : null}
        {draft.privilegedRelease && !draft.privilegedReleaseConfirmed ? <p className="notice notice--danger" role="alert">开启特权发布前必须确认 root 信任边界。</p> : null}
      </section>
      {error ? <div className="form-span"><ApiErrorNotice error={toNotice(error)} /></div> : null}
      {error ? <p className="notice form-span">部署目标创建失败不会回滚：应用与来源已保留，可修正后重试或跳过目标；应用入口：{appLink}。</p> : null}
      <div className="form-actions form-span"><Button type="button" onClick={onBack}>上一步</Button><Button type="button" onClick={onSkip}>跳过目标</Button><Button tone="primary" disabled={pending || !selectedNode || !source}>{pending ? "正在创建目标..." : "创建目标"}</Button></div>
    </form>
  </section>;
}

function DoneStep({ app, source, target, envExamples, onRestart }: {
  app: ApplicationResponse | null;
  source: ApplicationSourceResponse | null;
  target: DeploymentTargetResponse | null;
  envExamples: ReturnType<typeof templateEnvExamples> | null;
  onRestart(): void;
}) {
  return <section className="wizard-panel wizard-done">
    <div className="wizard-done__icon"><CheckCircle2 aria-hidden="true" /></div>
    <div className="section-heading"><div><h3>{target ? "应用与部署目标已创建" : "应用已创建"}</h3><p>下一步把模板文件推送到业务仓库、登记应用配置，然后发起部署。</p></div></div>
    <dl className="definition-grid">
      {app ? <div><dt>应用</dt><dd><Link className="text-link" to={`/apps/${app.id}`}>{app.name}</Link></dd></div> : null}
      {source ? <div><dt>Git 来源</dt><dd><code>{source.repositoryUrl}</code>{source.deploymentBranch ? <> · <code>{source.deploymentBranch}</code></> : null}</dd></div> : null}
      {target ? <div><dt>部署目标</dt><dd><Link className="text-link" to={`/apps/${target.applicationId}/targets/${target.id}`}>{target.nodeId}</Link></dd></div> : null}
    </dl>
    {envExamples ? <section className="wizard-env-examples">
      <div className="section-heading"><div><h4>Env 示例</h4><p>复制后到应用配置登记；密码使用真实值，禁止提交到仓库。</p></div></div>
      <ClipboardFallback value={envExamples.composeEnv} label="复制 compose.env 示例" />
      <ClipboardFallback value={envExamples.serviceEnv} label="复制服务 Env 示例" />
    </section> : null}
    <ol className="wizard-next-steps">
      <li>把模板目录复制到独立 Git 仓库并推送，再在应用详情刷新并固定分支。</li>
      <li>在应用配置登记 compose.env 与服务 Env，同步到目标节点。</li>
      <li>在应用详情创建部署并等待两阶段 release 完成。</li>
    </ol>
    <div className="form-actions">
      {app ? <Link className="button button--primary" to={`/apps/${app.id}`}>继续到应用详情</Link> : null}
      <Button type="button" onClick={onRestart}>再创建一个</Button>
    </div>
  </section>;
}
