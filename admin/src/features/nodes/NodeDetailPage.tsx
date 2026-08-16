import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { CheckCircle2, RefreshCw, ShieldX, TerminalSquare } from "lucide-react";
import { lazy, Suspense, useState, type FormEvent } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import type { AgentEnrollmentResponse } from "../../api/generated/models/AgentEnrollmentResponse";
import type { AgentInstallCommandResponse } from "../../api/generated/models/AgentInstallCommandResponse";
import type { NodeCheckResponse } from "../../api/generated/models/NodeCheckResponse";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { Field, Select, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { AGENT_ENVIRONMENTS, environmentLabel } from "../agents/environments";
import { agentsApi } from "../agents/api";
import { useAuth } from "../auth/AuthContext";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { statusLabel } from "./NodesPage";
import { nodesApi, terminalApi, type TerminalCapability } from "./api";
import { NodeTelemetrySection } from "./NodeTelemetrySection";

const NodeTerminal = lazy(() => import("./NodeTerminal").then((module) => ({ default: module.NodeTerminal })));

export function NodeDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["node", id], queryFn: () => nodesApi.nodesShow({ id }) });
  const agents = useQuery({
    queryKey: ["agents", "node-links"],
    queryFn: () => agentsApi.agentsList({ limit: 200 }),
    enabled: isAdministrator,
  });
  const terminalCapability = useQuery({
    queryKey: ["node", id, "terminal-capability"],
    queryFn: () => terminalApi.capability(id),
    enabled: isAdministrator,
  });
  const [check, setCheck] = useState<NodeCheckResponse | null>(null);
  const [adopting, setAdopting] = useState(false);
  const [adoptName, setAdoptName] = useState("");
  const [adoptEnvironment, setAdoptEnvironment] = useState("dev");
  const [enrollment, setEnrollment] = useState<AgentEnrollmentResponse | null>(null);
  const [command, setCommand] = useState<AgentInstallCommandResponse | null>(null);
  const [confirm, setConfirm] = useState<"command" | "revoke" | "archive" | null>(null);
  const linkedAgent = agents.data?.items.find((item) => item.nodeId === id);

  function secureContext() { if (!auth.csrfToken) throw new Error("缺少 CSRF token"); return auth.csrfToken; }
  const runCheck = useMutation({
    mutationFn: () => nodesApi.nodesRunCheck({ id, xCSRFToken: secureContext() }),
    onSuccess: (result) => {
      setCheck(result);
      void detail.refetch();
      void queryClient.invalidateQueries({ queryKey: ["nodes"] });
    },
  });
  const adopt = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return agentsApi.agentsCreate({ xCSRFToken: auth.csrfToken, createAgentRequest: { name: adoptName.trim(), environment: adoptEnvironment, nodeId: id } });
    },
    onSuccess: async (result) => {
      setEnrollment(result);
      setAdopting(false);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["agents", "node-links"] }),
        queryClient.invalidateQueries({ queryKey: ["agents"] }),
        queryClient.invalidateQueries({ queryKey: ["node", id] }),
        queryClient.invalidateQueries({ queryKey: ["nodes"] }),
      ]);
    },
  });
  const regenerate = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !linkedAgent) throw new Error("节点尚未关联协同程序");
      return agentsApi.agentsCreateInstallCommand({ agentId: linkedAgent.id, xCSRFToken: auth.csrfToken });
    },
    onSuccess: (result) => { setCommand(result); setConfirm(null); },
  });
  const revoke = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken || !linkedAgent) throw new Error("节点尚未关联协同程序");
      await agentsApi.agentsRevoke({ agentId: linkedAgent.id, xCSRFToken: auth.csrfToken });
    },
    onSuccess: async () => {
      setConfirm(null);
      setCommand(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["agents"] }),
        queryClient.invalidateQueries({ queryKey: ["node", id] }),
        queryClient.invalidateQueries({ queryKey: ["nodes"] }),
      ]);
    },
  });
  const archive = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      await nodesApi.nodesArchive({ id, xCSRFToken: auth.csrfToken });
    },
    onSuccess: () => {
      setConfirm(null);
      void queryClient.invalidateQueries({ queryKey: ["node", id] });
      void queryClient.invalidateQueries({ queryKey: ["nodes"] });
    },
  });
  const unarchive = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      await nodesApi.nodesUnarchive({ id, xCSRFToken: auth.csrfToken });
    },
    onSuccess: () => {
      setConfirm(null);
      void queryClient.invalidateQueries({ queryKey: ["node", id] });
      void queryClient.invalidateQueries({ queryKey: ["nodes"] });
    },
  });
  async function submitAdopt(event: FormEvent) { event.preventDefault(); await adopt.mutateAsync().catch(() => undefined); }

  if (detail.isLoading) return <PageState kind="loading" />;
  if (detail.isError || !detail.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(detail.error)} /><Link className="button button--default" to="/nodes">返回节点</Link></div>;
  const node = detail.data;
  const online = node.status === "online";
  const view = isAdministrator && searchParams.get("view") === "ssh" ? "ssh" : "overview";
  const selectView = (next: "overview" | "ssh") => {
    const params = new URLSearchParams(searchParams);
    if (next === "overview") params.delete("view");
    else params.set("view", "ssh");
    setSearchParams(params);
  };

  return <section className="workspace detail-page">
    <BackLink to="/nodes" parentLabel="节点列表" />
    <div className="detail-title"><div><h2>{node.name}</h2><p><code>{node.id}</code></p></div><div className="detail-title-badges"><span className={`status-badge status-badge--${online ? "online" : "offline"}`}>{statusLabel(node.status)}</span>{node.archivedAt ? <span className="status-badge status-badge--archived">已归档</span> : null}</div></div>
    <div className="detail-tabs" role="tablist" aria-label="节点详情视图">
      <button type="button" role="tab" aria-selected={view === "overview"} onClick={() => selectView("overview")}>概览</button>
      {isAdministrator ? <button type="button" role="tab" aria-selected={view === "ssh"} onClick={() => selectView("ssh")}>SSH</button> : null}
    </div>
    <div role="tabpanel" aria-label="概览" hidden={view !== "overview"}>
    {view === "overview" ? <>
    <dl className="definition-grid">
      <div><dt>工作根目录</dt><dd><code>{node.workRoot || "尚未上报"}</code></dd></div>
      <div><dt>Secrets root</dt><dd><code>{node.secretsRoot || "尚未上报"}</code></dd></div>
      <div><dt>最近检查</dt><dd>{node.checkedAt ? new Date(node.checkedAt).toLocaleString("zh-CN") : "尚未检查"}</dd></div>
    </dl>
    <NodeTelemetrySection nodeId={id} />
    {isAdministrator ? <section className="detail-section">
      <div className="section-head"><div><h3>节点协同程序</h3><p>协同程序维护节点身份、在线连接和部署任务执行。</p></div>{!linkedAgent ? <Button disabled={agents.isLoading || adopt.isPending} onClick={() => { setAdopting(true); setAdoptName(node.name); setAdoptEnvironment("dev"); }}><RefreshCw aria-hidden="true" />安装协同程序</Button> : null}</div>
      {agents.isError ? <ApiErrorNotice error={toNotice(agents.error)} /> : linkedAgent ? <>
        <dl className="definition-grid"><div><dt>环境</dt><dd>{environmentLabel(linkedAgent.environment)}</dd></div><div><dt>身份状态</dt><dd>{linkedAgent.revokedAt ? "已撤销" : "有效"}</dd></div><div><dt>版本</dt><dd><code>{linkedAgent.agentVersion ? `v${linkedAgent.agentVersion}` : "尚未上报"}</code></dd></div><div><dt>协议版本</dt><dd>{linkedAgent.protocolVersion ?? "尚未协商"}</dd></div><div><dt>主机</dt><dd>{linkedAgent.hostname ?? "尚未上报"}</dd></div><div><dt>架构</dt><dd>{linkedAgent.architecture ?? "尚未上报"}</dd></div><div><dt>最后在线</dt><dd>{linkedAgent.lastSeenAt ? new Date(linkedAgent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</dd></div><div><dt>协同程序 ID</dt><dd><code>{linkedAgent.id}</code></dd></div></dl>
        <div className="node-agent-actions"><Button onClick={() => setConfirm("command")}><RefreshCw aria-hidden="true" />重新生成安装命令</Button><Button tone="danger" disabled={Boolean(linkedAgent.revokedAt)} onClick={() => setConfirm("revoke")}><ShieldX aria-hidden="true" />{linkedAgent.revokedAt ? "身份已撤销" : "撤销节点身份"}</Button></div>
        {regenerate.error ? <ApiErrorNotice error={toNotice(regenerate.error)} /> : null}
        {revoke.error ? <ApiErrorNotice error={toNotice(revoke.error)} /> : null}
      </> : adopting ? <form className="inline-form" onSubmit={(event) => void submitAdopt(event)}>
        <Field label="节点名称"><TextInput autoFocus required minLength={1} maxLength={80} disabled={adopt.isPending} value={adoptName} onChange={(event) => setAdoptName(event.target.value)} /></Field>
        <Field label="环境"><Select required disabled={adopt.isPending} value={adoptEnvironment} onChange={(event) => setAdoptEnvironment(event.target.value)}>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></Field>
        {adopt.error ? <ApiErrorNotice error={toNotice(adopt.error)} /> : null}
        <div className="form-actions"><Button type="button" disabled={adopt.isPending} onClick={() => setAdopting(false)}>取消</Button><Button tone="primary" disabled={adopt.isPending}>{adopt.isPending ? "正在生成..." : "生成安装命令"}</Button></div>
      </form> : <p className="muted">该节点尚未安装协同程序。</p>}
      {enrollment ? <section className="agent-command" aria-live="polite"><div className="section-heading"><div><h3>节点安装命令</h3><p>命令包含一次性 token，请在 {new Date(enrollment.enrollmentExpiresAt).toLocaleString("zh-CN")} 前到目标 Linux 服务器执行。</p></div><Button onClick={() => setEnrollment(null)}>关闭</Button></div><ClipboardFallback value={enrollment.installCommand} label="复制命令" failure="自动复制失败，请选中完整命令后手动复制。" /></section> : null}
      {command ? <section className="agent-command" aria-live="polite"><div className="section-heading"><div><h3>新的安装命令</h3><p>此前未使用的安装命令已失效，请在 {new Date(command.enrollmentExpiresAt).toLocaleString("zh-CN")} 前执行新命令。</p></div><Button onClick={() => setCommand(null)}>关闭</Button></div><ClipboardFallback value={command.installCommand} label="复制命令" /></section> : null}
    </section> : <section className="detail-section"><h3>节点状态</h3><p>普通用户可查看已授权节点状态；协同程序接入和维护由管理员完成。</p></section>}
    {isAdministrator ? <section className="detail-section">
      <div className="section-head"><div><h3>节点能力检查</h3><p>通过协同程序检查系统、架构、工作目录和可用磁盘，不执行部署脚本。</p></div><Button disabled={!linkedAgent || !online || runCheck.isPending} onClick={() => runCheck.mutate()}>{runCheck.isPending ? "正在检查..." : "执行检查"}</Button></div>
      {!linkedAgent ? <p className="muted">安装协同程序后才能执行检查。</p> : !online ? <p className="muted">节点离线，恢复连接后才能执行检查。</p> : null}
      {check ? <CheckResult value={check} /> : null}
      {runCheck.error ? <ApiErrorNotice error={toNotice(runCheck.error)} /> : null}
    </section> : null}
    {isAdministrator ? <section className="detail-section">
      <div className="section-head"><div><h3>节点生命周期</h3><p>{node.archivedAt ? `节点已于 ${formatTime(node.archivedAt)} 归档，归档节点不参与部署调度、能力检查和终端连接；恢复后重新参与调度。` : "归档节点不再参与部署调度、能力检查和终端连接，历史部署记录与归档前数据保留。"}</p></div>{node.archivedAt ? <Button disabled={unarchive.isPending} onClick={() => setConfirm("archive")}>{unarchive.isPending ? "正在恢复..." : "恢复节点"}</Button> : <Button tone="danger" disabled={archive.isPending} onClick={() => setConfirm("archive")}>{archive.isPending ? "正在归档..." : "归档节点"}</Button>}</div>
      {archive.error ? <ApiErrorNotice error={toNotice(archive.error)} /> : null}
      {unarchive.error ? <ApiErrorNotice error={toNotice(unarchive.error)} /> : null}
    </section> : null}
    {linkedAgent ? <ConfirmDialog open={confirm !== null} title={confirm === "revoke" ? `撤销 ${node.name} 的节点身份？` : confirm === "archive" ? (node.archivedAt ? `恢复 ${node.name} 节点？` : `归档 ${node.name} 节点？`) : "重新生成安装命令？"} message={confirm === "revoke" ? "在线连接会立即关闭，恢复时必须使用新命令重新绑定。" : confirm === "archive" ? (node.archivedAt ? "恢复后节点重新参与部署调度，历史记录不受影响。" : "归档后节点不再接收新的部署、检查和终端连接；进行中的部署会阻止归档。历史部署记录保留，可随时恢复。") : "此前尚未使用的安装命令将立即失效。"} confirmLabel={confirm === "revoke" ? "确认撤销" : confirm === "archive" ? (node.archivedAt ? "确认恢复" : "确认归档") : "确认重新生成"} tone={confirm === "revoke" || (confirm === "archive" && !node.archivedAt) ? "danger" : "primary"} pending={revoke.isPending || regenerate.isPending || archive.isPending || unarchive.isPending} onClose={() => setConfirm(null)} onConfirm={() => { if (confirm === "revoke") revoke.mutate(); else if (confirm === "archive") { if (node.archivedAt) unarchive.mutate(); else archive.mutate(); } else regenerate.mutate(); }} /> : null}
    </> : null}
    </div>
    {isAdministrator ? <div role="tabpanel" aria-label="SSH" hidden={view !== "ssh"}>
      {view === "ssh" ? <TerminalPanel
        nodeId={id}
        nodeName={node.name}
        csrfToken={auth.csrfToken}
        capability={terminalCapability}
      /> : null}
    </div> : null}
  </section>;
}

function TerminalPanel({ nodeId, nodeName, csrfToken, capability }: {
  nodeId: string;
  nodeName: string;
  csrfToken: string | null;
  capability: UseQueryResult<TerminalCapability>;
}) {
  if (capability.isLoading) return <PageState kind="loading" />;
  if (capability.isError || !capability.data) return <ApiErrorNotice error={toNotice(capability.error)} />;
  if (!capability.data.available) {
    return <section className="terminal-gate" aria-live="polite">
      <span className="terminal-gate__icon"><TerminalSquare aria-hidden="true" /></span>
      <div><h3>{terminalGateMessage(capability.data.unavailableCode)}</h3><p>{terminalGateHelp(capability.data.unavailableCode)}</p></div>
    </section>;
  }
  if (!csrfToken) return <ApiErrorNotice error={toNotice(new Error("缺少 CSRF token"))} />;
  return <Suspense fallback={<PageState kind="loading" />}><NodeTerminal nodeId={nodeId} nodeName={nodeName} csrfToken={csrfToken} capability={capability.data} /></Suspense>;
}

function terminalGateMessage(code: string | null) {
  const messages: Record<string, string> = {
    terminal_agent_identity_invalid: "节点 Agent 身份无效或已撤销",
    terminal_agent_offline: "节点 Agent 当前离线",
    terminal_protocol_unsupported: "Agent 版本不支持终端",
    terminal_executor_unavailable: "节点终端 executor 不可用",
  };
  return code ? messages[code] ?? "节点终端当前不可用" : "节点终端当前不可用";
}

function terminalGateHelp(code: string | null) {
  const messages: Record<string, string> = {
    terminal_agent_identity_invalid: "重新绑定有效的 Agent 身份后再连接。",
    terminal_agent_offline: "等待 Agent 恢复在线连接后再试。",
    terminal_protocol_unsupported: "重新安装 Agent v11 后再连接。",
    terminal_executor_unavailable: "安装并启动与 Agent 配套的 root executor。",
  };
  return code ? messages[code] ?? "请检查节点与 Agent 状态。" : "请检查节点与 Agent 状态。";
}

function CheckResult({ value }: { value: NodeCheckResponse }) {
  if (value.status === "running") return <div className="check-result"><strong>检查任务已下发</strong><p>Agent 正在执行 SystemInspect，完成后节点信息会自动更新。</p></div>;
  if (value.status !== "succeeded") return <div className="check-result check-result--failed" role="alert"><strong>检查失败：{value.failureCode ?? "unknown"}</strong><p>{value.failureMessage ?? "请核对 Agent 状态后重试。"}</p></div>;
  return <dl className="check-result"><div><dt>系统</dt><dd>{value.osName ?? "-"}</dd></div><div><dt>架构</dt><dd>{value.architecture ?? "-"}</dd></div><div><dt>可用磁盘</dt><dd>{formatBytes(value.diskAvailableBytes)}</dd></div><div><dt>结果</dt><dd className="success-text"><CheckCircle2 aria-hidden="true" />检查通过</dd></div></dl>;
}
function formatBytes(value?: number | null) { if (value == null) return "-"; return `${(value / 1024 / 1024 / 1024).toFixed(1)} GiB`; }
function formatTime(value: string) { try { return new Date(value).toLocaleString("zh-CN"); } catch { return value; } }
