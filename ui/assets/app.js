(function bootstrapDeployGoUI() {
  "use strict";

  const root = document.getElementById("app");
  const source = window.DeployGoMock;
  if (!root || !source) return;

  const STORAGE_KEY = "deploy-go-ui/design-source-v1";
  let saved = {};
  try { saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}"); } catch { localStorage.removeItem(STORAGE_KEY); }
  const state = {
    scenario: saved.scenario || "running",
    webDeploymentFilter: saved.webDeploymentFilter || saved.deploymentFilter || "all",
    mobileDeploymentFilter: saved.mobileDeploymentFilter || "all",
    query: saved.query || "",
    selectedAppId: saved.selectedAppId || "atlas-api",
    selectedTarget: saved.selectedTarget || "prod-cn-1",
    logFollowing: true,
    modal: null,
    toast: null,
    createdDeployment: saved.createdDeployment || null,
    canceledIds: new Set(saved.canceledIds || []),
    disabledUserIds: new Set(saved.disabledUserIds || ["xu-yan"]),
    createdUsers: saved.createdUsers || [],
    userOverrides: saved.userOverrides || [],
    createdApps: saved.createdApps || [],
    createdNodes: saved.createdNodes || [],
    createdAgents: saved.createdAgents || [],
    revokedAgentIds: new Set(saved.revokedAgentIds || []),
    agentCommand: null,
    agentCreating: false,
    createdTargets: saved.createdTargets || [],
    applicationGrants: saved.applicationGrants || source.grants || {},
    setupComplete: saved.setupComplete !== false,
    authenticated: saved.authenticated !== false,
    role: saved.role || "admin",
    resourceQueries: saved.resourceQueries || { apps: "", nodes: "" },
    resourceStatuses: saved.resourceStatuses || { apps: "all", nodes: "all" },
    environmentFilter: saved.environmentFilter || "all",
    appFilter: saved.appFilter || "all",
    nodeFilter: saved.nodeFilter || "all",
    checkingNodeIds: new Set(),
    retrySourceId: saved.retrySourceId || null,
    preferences: saved.preferences || { failed: true, completed: true, node: true },
    contractCheckStatus: "idle",
    targetContractCheckStatus: "idle",
    mobileQueries: saved.mobileQueries || { deployments: "", apps: "", nodes: "" },
    loginError: false,
    nodeCheckResults: saved.nodeCheckResults || {},
    systemSettings: saved.systemSettings || { concurrency: "queue", timeout: "20", retention: "90" },
    auditEvents: saved.auditEvents || [],
    cancelingIds: new Set(saved.cancelingIds || []),
    tasks: {},
    dirtyForm: null,
    pendingNavigation: null,
    navigationBypass: false,
    currentRoute: (window.location.hash.replace(/^#/, "") || "/entry").replace(/\/$/, "") || "/entry",
    focusToken: null,
    visibleCounts: { webDeployments: 8, mobileDeployments: 6, mobileApps: 6, mobileNodes: 6 },
    logToolError: "",
  };

  const icons = {
    overview: '<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/>',
    deploy: '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/>',
    app: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8 9h8M8 13h5M8 17h8"/>',
    node: '<rect x="4" y="3" width="16" height="7" rx="2"/><rect x="4" y="14" width="16" height="7" rx="2"/><path d="M8 6.5h.01M8 17.5h.01M12 6.5h5M12 17.5h5"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.86 2.86-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1v.1H9.5V21a1.7 1.7 0 0 0-1.1-1.6 1.7 1.7 0 0 0-1.88.34l-.06.06-2.86-2.86.06-.06A1.7 1.7 0 0 0 4 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1-.4h-.1V9.5h.1A1.7 1.7 0 0 0 4 8.4a1.7 1.7 0 0 0-.34-1.88l-.06-.06L6.46 3.6l.06.06A1.7 1.7 0 0 0 8.4 4a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1v-.1h4.1v.1A1.7 1.7 0 0 0 15 4a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.86 2.86-.06.06A1.7 1.7 0 0 0 19.4 8c.2.4.6.8 1 1 .3.2.7.3 1 .3h.1v4.1h-.1a1.7 1.7 0 0 0-1.6 1.1Z"/>',
    profile: '<circle cx="12" cy="8" r="4"/><path d="M4.5 21a7.5 7.5 0 0 1 15 0"/>',
    shield: '<path d="M12 3 19 6v5c0 4.6-2.8 8-7 10-4.2-2-7-5.4-7-10V6l7-3Z"/><path d="m9 12 2 2 4-4"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    arrow: '<path d="M5 12h14M13 6l6 6-6 6"/>',
    back: '<path d="m15 18-6-6 6-6"/>',
    pause: '<path d="M9 5v14M15 5v14"/>',
    play: '<path d="m8 5 11 7-11 7Z"/>',
    copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
    down: '<path d="M12 4v16M6 14l6 6 6-6"/>',
    check: '<path d="m5 12 4 4L19 6"/>',
    alert: '<path d="M10.3 3.4 2.7 17a2 2 0 0 0 1.7 3h15.2a2 2 0 0 0 1.7-3L13.7 3.4a2 2 0 0 0-3.4 0Z"/><path d="M12 9v4M12 17h.01"/>',
    x: '<path d="m6 6 12 12M18 6 6 18"/>',
    external: '<path d="M14 3h7v7M10 14 21 3"/><path d="M21 14v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5"/>',
  };

  const statusLabels = {
    running: "运行中", success: "成功", succeeded: "成功", failed: "失败", queued: "排队中", canceled: "已取消", interrupted: "执行中断",
    canceling: "取消中", healthy: "正常", deploying: "部署中", error: "异常", archived: "已归档",
    online: "在线", offline: "离线", checking: "检查中", disabled: "已停用",
  };

  const managedUsers = [
    { id: "chen-zhou", name: "陈舟", email: "chen@deploy.go", role: "管理员", admin: true, lastActive: "刚刚" },
    { id: "lin-zhen", name: "林臻", email: "lin@deploy.go", role: "普通用户", lastActive: "今天 13:06" },
    { id: "zhou-ning", name: "周宁", email: "zhou@deploy.go", role: "普通用户", lastActive: "昨天 20:17" },
    { id: "xu-yan", name: "徐言", email: "xu@deploy.go", role: "普通用户", lastActive: "7 月 18 日" },
  ];
  function mergeById(base, overrides) {
    const overrideMap = new Map(overrides.map((item) => [item.id, item]));
    return [...base.map((item) => overrideMap.get(item.id) || item), ...overrides.filter((item) => !base.some((sourceItem) => sourceItem.id === item.id))];
  }
  function upsertById(items, value) {
    const existing = items.find((item) => item.id === value.id);
    existing ? Object.assign(existing, value) : items.push(value);
  }
  function allManagedUsers() { return [...mergeById(managedUsers, state.userOverrides), ...state.createdUsers]; }
  function allApps() { return mergeById(source.apps, state.createdApps); }
  function allNodes() { return mergeById(source.nodes, state.createdNodes); }
  function allAgents() { return [...source.agents, ...state.createdAgents].map((agent) => state.revokedAgentIds.has(agent.id) ? { ...agent, status: "offline", revoked: true } : agent); }
  function allTargets() {
    const generated = allApps().map((app) => ({ id: app.target, appId: app.id, environment: app.environment, nodeId: app.nodeId, script: app.script, args: app.args || "--environment production", secretRef: app.secretRef || "secret/prod/deploy-token", timeout: app.timeout || "20", health: app.health || "/health", successCode: "200", contract: app.id === "billing-worker" ? "failed" : "valid" }));
    const key = (target) => `${target.appId}:${target.id}`;
    const overrides = new Map(state.createdTargets.map((target) => [key(target), target]));
    return [...generated.map((target) => overrides.get(key(target)) || target), ...state.createdTargets.filter((target) => !generated.some((item) => key(item) === key(target)))];
  }
  function grantedApps(userId) { return new Set(state.applicationGrants[userId] || []); }
  function isAdmin() { return state.role === "admin"; }

  function icon(name, label) {
    return `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${icons[name] || icons.app}</svg>${label ? `<span>${label}</span>` : ""}`;
  }

  function status(value) {
    const effective = value?.id ? deploymentStatus(value) : (value.status || value);
    return `<span class="status status--${effective}">${statusLabels[effective] || effective}</span>`;
  }
  function deploymentStatus(deployment) {
    if (state.canceledIds.has(deployment.id)) return "canceled";
    if (state.cancelingIds.has(deployment.id)) return "canceling";
    return deployment.status === "succeeded" ? "success" : deployment.status;
  }

  function findApp(id) { return allApps().find((item) => item.id === id) || null; }
  function findNode(id) { return allNodes().find((item) => item.id === id) || null; }
  function findDeployment(id) {
    if (state.createdDeployment && state.createdDeployment.id === id) return state.createdDeployment;
    const direct=source.deployments.find((item) => item.id === id); if(direct)return state.scenario==="interrupted"&&id==="dep-1042"?{...direct,status:"interrupted"}:direct;
    const denseMatch=id.match(/^(dep-\d+)-(\d+)$/); if(state.scenario==="dense"&&denseMatch){const original=source.deployments.find(item=>item.id===denseMatch[1]);const round=Number(denseMatch[2]);const index=source.deployments.findIndex(item=>item.id===denseMatch[1]);if(original&&index>=0)return{...original,id,number:`#${1042-round*6-index}`};}
    return null;
  }
  function appById(id) { return findApp(id) || allApps()[0]; }
  function nodeById(id) { return findNode(id) || allNodes()[0]; }
  function deploymentById(id) {
    return findDeployment(id) || source.deployments[0];
  }

  function scenarioData() {
    const base = { apps: allApps(), nodes: allNodes(), deployments: [...source.deployments] };
    if (state.createdDeployment) base.deployments.unshift(state.createdDeployment);
    if (state.scenario === "empty") return { apps: [], nodes: [], deployments: [] };
    if (state.scenario === "healthy") {
      base.nodes = base.nodes.filter((n) => n.status === "online");
      base.apps = base.apps.filter((a) => a.status === "healthy" || a.status === "deploying");
      base.deployments = base.deployments.filter((d) => d.status === "success" || d.status === "running");
    }
    if (state.scenario === "failed") base.deployments = [source.deployments[2], source.deployments[0], source.deployments[1]];
    if (state.scenario === "contract-failed") base.apps = [source.apps[2],source.apps[0],source.apps[1]];
    if (state.scenario === "interrupted") base.deployments = base.deployments.map((deployment) => deployment.id === "dep-1042" ? { ...deployment, status: "interrupted" } : deployment);
    if (state.scenario === "dense") {
      base.deployments = Array.from({ length: 4 }, (_, round) => source.deployments.map((d, i) => ({ ...d, id: `${d.id}-${round}`, number: `#${1042 - round * 6 - i}` }))).flat();
    }
    return base;
  }

  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      scenario: state.scenario,
      role: state.role,
      authenticated: state.authenticated,
      preferences: state.preferences,
      selectedAppId: state.selectedAppId,
      selectedTarget: state.selectedTarget,
      webDeploymentFilter: state.webDeploymentFilter,
      mobileDeploymentFilter: state.mobileDeploymentFilter,
      query: state.query,
      resourceQueries: state.resourceQueries,
      resourceStatuses: state.resourceStatuses,
      environmentFilter: state.environmentFilter,
      appFilter: state.appFilter,
      nodeFilter: state.nodeFilter,
      mobileQueries: state.mobileQueries,
      createdDeployment: state.createdDeployment,
      retrySourceId: state.retrySourceId,
      canceledIds: [...state.canceledIds],
      cancelingIds: [...state.cancelingIds],
      disabledUserIds: [...state.disabledUserIds],
      createdUsers: state.createdUsers,
      userOverrides: state.userOverrides,
      createdApps: state.createdApps,
      createdNodes: state.createdNodes,
      createdAgents: state.createdAgents,
      revokedAgentIds: [...state.revokedAgentIds],
      createdTargets: state.createdTargets,
      applicationGrants: state.applicationGrants,
      setupComplete: state.setupComplete,
      nodeCheckResults: state.nodeCheckResults,
      systemSettings: state.systemSettings,
      auditEvents: state.auditEvents,
    }));
  }
  function recordAudit(action, object, result = "成功") {
    state.auditEvents.unshift(["刚刚", isAdmin() ? "陈舟" : "林臻", action, object, result]);
    state.auditEvents = state.auditEvents.slice(0, 20);
  }
  function go(path) {
    state.navigationBypass = true;
    window.location.hash = `#${path}`;
  }
  function routePath() { return (window.location.hash.replace(/^#/, "") || "/entry").replace(/\/$/, "") || "/entry"; }

  function focusToken(element) {
    if (!element) return null;
    if (element.id) return `#${CSS.escape(element.id)}`;
    const action = element.dataset?.action;
    const id = element.dataset?.id;
    if (action) return `[data-action="${CSS.escape(action)}"]${id ? `[data-id="${CSS.escape(id)}"]` : ""}`;
    const href = element.getAttribute?.("href");
    if (href) return `[href="${CSS.escape(href)}"]`;
    return null;
  }

  function restoreFocus(token) {
    if (!token) return;
    requestAnimationFrame(() => root.querySelector(token)?.focus());
  }

  function task(key) { return state.tasks[key] || { status: "idle", error: "" }; }
  function isPending(key) { return task(key).status === "pending"; }
  function runTask(key, work, options = {}) {
    if (isPending(key)) return;
    state.tasks[key] = { status: "pending", error: "" };
    render();
    window.setTimeout(() => {
      if (state.scenario === "operation-failed" || options.fail === true) {
        state.tasks[key] = { status: "failed", error: options.error || "操作没有完成，请重试。" };
        render();
        return;
      }
      try {
        work();
      } catch (error) {
        state.tasks[key] = { status: "failed", error: error instanceof Error ? error.message : "操作没有完成，请重试。" };
        render();
        return;
      }
      state.tasks[key] = { status: "succeeded", error: "" };
      if (options.toast) state.toast = options.toast;
      persist();
      render();
      if (options.after) options.after();
    }, options.delay || 320);
  }

  function inlineTask(key) {
    const current = task(key);
    if (current.status !== "failed") return "";
    return `<div class="action-error" role="alert">${icon("alert")} ${current.error}</div>`;
  }

  const appParentRoutes = [
    [/^\/app\/deployments\/(?:new|[^/]+)$/, "/app/deployments"],
    [/^\/app\/apps\/[^/]+$/, "/app/resources"],
    [/^\/app\/nodes\/[^/]+$/, "/app/resources"],
    [/^\/app\/mine\/users\/(?:new|[^/]+)$/, "/app/mine/users"],
    [/^\/app\/mine\/users$/, "/app/mine"],
    [/^\/app\/mine\/(?:profile|preferences|about)$/, "/app/mine"],
  ];
  function appParent(path) { return appParentRoutes.find(([pattern]) => pattern.test(path))?.[1] || "/app/overview"; }

  function requestNavigation(path, trigger = null) {
    if (state.dirtyForm && state.dirtyForm.route === routePath()) {
      state.focusToken = focusToken(trigger || document.activeElement);
      state.pendingNavigation = path;
      state.modal = { type: "discard" };
      render();
      return;
    }
    go(path);
  }

  function markDirty(form) {
    if (!form || form.matches("[data-login-form]")) return;
    state.dirtyForm = { route: routePath() };
  }

  function clearDirty() {
    state.dirtyForm = null;
    state.pendingNavigation = null;
  }

  function validationMessage(input) {
    const value = input.value.trim();
    if (input.required && !value) return "此项为必填项。";
    if (input.type === "email" && value && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return "请输入有效的邮箱地址。";
    if (input.name === "port" && (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65535)) return "端口必须是 1 到 65535 之间的整数。";
    if (["directory", "script", "health"].includes(input.name) && value && !value.startsWith("/")) return "请输入以 / 开头的绝对路径。";
    if (input.name === "successCode" && value && (!/^\d{3}$/.test(value) || Number(value) < 100 || Number(value) > 599)) return "请输入有效的 HTTP 状态码。";
    if (input.minLength > 0 && value.length < input.minLength) return `至少输入 ${input.minLength} 个字符。`;
    return "";
  }

  function validateForm(element) {
    element.querySelectorAll(".field-error, .form-error-summary").forEach((node) => node.remove());
    element.querySelectorAll("[aria-invalid]").forEach((input) => input.removeAttribute("aria-invalid"));
    const errors = [...element.querySelectorAll("input, select, textarea")]
      .map((input) => ({ input, message: validationMessage(input) }))
      .filter((item) => item.message);
    if (!errors.length) return true;
    const summary = document.createElement("div");
    summary.className = "form-error-summary";
    summary.setAttribute("role", "alert");
    summary.tabIndex = -1;
    summary.innerHTML = `<strong>请修正 ${errors.length} 项内容后重试</strong><ul>${errors.map(({ input, message }) => `<li><a href="#${input.id}">${input.labels?.[0]?.textContent || input.name}：${message}</a></li>`).join("")}</ul>`;
    element.prepend(summary);
    errors.forEach(({ input, message }, index) => {
      input.setAttribute("aria-invalid", "true");
      const error = document.createElement("p");
      error.className = "field-error";
      error.id = `${input.id || `field-${index}`}-error`;
      error.textContent = message;
      input.setAttribute("aria-describedby", error.id);
      input.closest(".field")?.append(error);
    });
    errors[0].input.focus();
    return false;
  }

  function invalidateDependentCheck(form) {
    if (form.matches("[data-node-form]") && state.nodeTestStatus !== "idle") state.nodeTestStatus = "idle";
    else if (form.matches("[data-app-form]") && state.contractCheckStatus !== "idle") state.contractCheckStatus = "idle";
    else if (form.matches("[data-target-form]") && state.targetContractCheckStatus !== "idle") state.targetContractCheckStatus = "idle";
    else return;
    const panel = form.querySelector(".check-panel");
    panel?.classList.remove("is-success", "is-failed");
    const copy = panel?.querySelector("div");
    if (copy) copy.innerHTML = "<strong>配置已修改，需要重新检查</strong><p>旧检查结果已失效，保存前请重新执行检查。</p>";
    const submit = form.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
  }

  function sourceToolbar() {
    return `<div class="source-toolbar" aria-label="设计源工具">
      <select data-action="scenario" aria-label="Mock 数据场景">${source.scenarios.map((s) => `<option value="${s.id}" ${s.id === state.scenario ? "selected" : ""}>${s.label}</option>`).join("")}</select>
      <select data-action="role" aria-label="预览身份"><option value="admin" ${isAdmin() ? "selected" : ""}>管理员</option><option value="user" ${!isAdmin() ? "selected" : ""}>普通用户</option></select>
      <a class="source-toolbar__link" href="#/entry" title="设计源入口" aria-label="设计源入口">${icon("overview")}</a>
      <a class="source-toolbar__link" href="#/spec" title="设计规范" aria-label="设计规范">${icon("settings")}</a>
    </div>`;
  }

  function renderEntry() {
    return `<div class="entry">
      <header class="entry__bar"><div class="brand"><span class="brand__mark">DG</span><span>Deploy Go</span></div><span class="muted">UI Design Source</span></header>
      <main class="entry__main">
        <p class="eyebrow">Automation control surface</p>
        <h1>把脚本执行变成<br>清晰的部署过程。</h1>
        <p class="entry__lead">统一设计节点、应用、部署和日志的管理体验。Web 负责高密度配置，App 负责快速查看与操作。</p>
        <div class="entry__grid">
          <article class="entry-card"><span class="resource-mark">WEB</span><h2>Web 管理端</h2><p>完整工作台，适合筛选部署历史、配置资源和阅读长日志。</p><a class="btn btn--primary" href="#/web/overview">打开 Web 预览 ${icon("arrow")}</a></article>
          <article class="entry-card"><span class="resource-mark">APP</span><h2>App 管理端</h2><p>独立移动信息架构，聚焦状态、部署确认和过程跟踪。</p><a class="btn" href="#/app/overview">打开 App 预览 ${icon("arrow")}</a></article>
          <article class="entry-card"><span class="resource-mark">SYS</span><h2>设计规范</h2><p>查看 tokens、状态、控件和跨端组件的基础契约。</p><a class="btn" href="#/spec">查看规范 ${icon("arrow")}</a></article>
        </div>
      </main>
      ${sourceToolbar()}
    </div>`;
  }

  function renderSpec() {
    return `<div class="spec-page">
      <header class="spec-head"><p class="eyebrow">Deploy Go / Foundation</p><h1>设计规范</h1><p>低噪音的运行控制界面。结构依靠边界、间距和排版建立，颜色只承担动作与状态语义。</p></header>
      <main class="spec-content">
        <div class="section-head"><div><h2>语义色</h2><p>同一状态必须同时包含文字、图标或形状提示。</p></div></div>
        <div class="swatches"><div class="swatch" style="background:#0d1117;color:white">Navigation</div><div class="swatch" style="background:#24292f;color:white">Action</div><div class="swatch" style="background:#dafbe1">Success</div><div class="swatch" style="background:#fff8c5">Running</div><div class="swatch" style="background:#ffebe9">Failure</div><div class="swatch" style="background:#f0f2f4">Offline</div></div>
        <div class="section-head"><div><h2>部署状态</h2><p>列表、详情与 App 使用同一套状态名称。</p></div></div>
        <div class="component-row">${["queued","running","success","failed","canceling","canceled"].map(status).join("")}</div>
        <div class="section-head"><div><h2>操作控件</h2><p>明确命令使用文字按钮，工具操作使用图标与 tooltip。</p></div></div>
        <div class="component-row"><button class="btn btn--primary">${icon("deploy")} 发起部署</button><button class="btn">次要操作</button><button class="btn btn--danger">取消部署</button><button class="icon-btn" title="复制">${icon("copy")}</button><button class="btn" disabled>不可用</button></div>
        <div class="section-head"><div><h2>日志工作区</h2><p>连续表面、稳定工具条、等宽内容。</p></div></div>
        ${renderLogPanel(source.deployments[0])}
      </main>${sourceToolbar()}
    </div>`;
  }

  const navItems = [
    ["overview", "概览", "/web/overview"], ["deploy", "部署", "/web/deployments"],
    ["app", "应用", "/web/apps"], ["node", "节点", "/web/nodes"], ["agent", "Agent", "/web/agents"], ["settings", "设置", "/web/settings"],
  ];
  const settingsNavItems = [
    ["general", "系统设置", "/web/settings", "settings"],
    ["users", "用户管理", "/web/settings/users", "profile"],
    ["audit", "审计记录", "/web/settings/audit", "shield"],
  ];

  function webShell(content, active, title, subtitle, actions = "") {
    const visibleNav=isAdmin()?navItems:navItems.filter(([id])=>!["agent","settings"].includes(id));
    const path=routePath();
    const settingsSection=path.startsWith("/web/settings/users")?"users":path==="/web/settings/audit"?"audit":"general";
    const settingsNav=active==="settings"&&isAdmin()?`<nav class="sidebar-subnav" aria-label="设置导航">${settingsNavItems.map(([id,label,target,iconName])=>`<a class="sidebar-subnav__link ${settingsSection===id?"is-active":""}" href="#${target}" title="${label}" ${settingsSection===id?'aria-current="page"':""}>${icon(iconName)}<span>${label}</span></a>`).join("")}</nav>`:"";
    return `<div class="web-shell">
      <aside class="sidebar"><a class="brand" href="#/entry"><span class="brand__mark">DG</span><span>Deploy Go</span></a>
        <nav aria-label="主导航">${visibleNav.map(([i,l,p]) => `<a class="nav-link ${active === i ? "is-active" : ""}" href="#${p}" ${active===i&&i!=="settings"?'aria-current="page"':""} ${i==="settings"&&active===i?'aria-expanded="true"':""}>${icon(i)}<span>${l}</span>${i === "deploy" ? '<b class="nav-link__badge">2</b>' : ""}</a>`).join("")}</nav>
        ${settingsNav}
        <div class="sidebar__footer"><button class="sidebar__user" data-action="signout"><span class="avatar">${isAdmin()?"陈":"林"}</span><span><strong>${isAdmin()?"陈舟":"林臻"}</strong><span class="subtle">${isAdmin()?"管理员":"普通用户"}</span></span></button></div>
      </aside>
      <main class="web-main"><header class="page-head"><div><h1>${title}</h1><p>${subtitle}</p></div><div class="page-head__actions">${actions}</div></header><div class="page-content">${content}</div></main>
      ${sourceToolbar()}${renderOverlay()}
    </div>`;
  }

  function emptyState(kind, mobile = false) {
    const noun = kind === "nodes" ? "节点" : kind === "apps" ? "应用" : "部署记录";
    const webPath=kind === "nodes" ? "agents" : kind === "apps" ? "apps/new" : "deployments/new";
    const mobilePath=kind === "deployments"?"deployments/new":"overview";
    const action=mobile?(kind==="deployments"?"发起部署":"返回概览"):(kind==="nodes"?"创建 Agent":`创建${noun}`);
    return `<div class="empty"><span class="empty__icon">${icon(kind === "nodes" ? "node" : kind === "apps" ? "app" : "deploy")}</span><h2>还没有${noun}</h2><p>${kind === "nodes" ? "先接入一台节点，后续应用才能配置部署目标。" : kind === "apps" ? "创建应用并配置脚本入口后，就可以发起首次部署。" : "发起一次部署后，执行过程和结果会显示在这里。"}</p><a class="btn btn--primary" href="#/${mobile?`app/${mobilePath}`:`web/${webPath}`}">${icon(mobile&&kind!=="deployments"?"back":"plus")} ${action}</a></div>`;
  }

  function renderWebOverview() {
    const data = scenarioData();
    if (!data.nodes.length) return webShell(emptyState("nodes"), "overview", "概览", "今天的部署运行状态", `<a class="btn btn--primary" href="#/web/agents">${icon("plus")} 创建 Agent</a>`);
    const running = data.deployments.filter((d) => ["running","queued"].includes(deploymentStatus(d))).length;
    const failed = data.deployments.filter((d) => deploymentStatus(d) === "failed").length;
    const offline = data.nodes.filter((n) => n.status === "offline").length;
    const recent = data.deployments.slice(0, 4);
    return webShell(`<div class="metric-row">
      <div class="metric"><span class="metric__label">运行中的部署</span><div class="metric__value">${running}<span class="metric__note">任务</span></div></div>
      <div class="metric"><span class="metric__label">今日成功率</span><div class="metric__value">96.4<span class="metric__note">%</span></div></div>
      <div class="metric"><span class="metric__label">失败待处理</span><div class="metric__value">${failed}<span class="metric__note">记录</span></div></div>
      <div class="metric"><span class="metric__label">异常节点</span><div class="metric__value">${offline}<span class="metric__note">/ ${data.nodes.length}</span></div></div>
    </div><div class="dashboard-grid"><section><div class="section-head"><div><h2>最近活动</h2><p>应用脚本的执行结果</p></div><a href="#/web/deployments" class="muted">查看全部</a></div><div class="activity-list">${recent.map((d) => { const app = appById(d.appId); return `<a class="activity-row" href="#/web/deployments/${d.id}"><span class="activity-row__dot ${d.status === "failed" ? "activity-row__dot--danger" : ""}"></span><div><strong>${app.name} ${d.version}</strong><span class="muted">${d.number} · ${nodeById(d.nodeId).name} · ${d.actor}</span></div><div>${status(d)}<div class="subtle" style="margin-top:5px;text-align:right">${d.createdAt}</div></div></a>`; }).join("")}</div></section>
      <aside><div class="section-head"><div><h2>需要关注</h2><p>影响部署条件的异常</p></div></div><div class="alert-list"><a class="alert-item" href="#/web/nodes/node-hz-01"><strong>${icon("alert")} hz-staging-01 离线</strong><p>最后连接于 8 分钟前，影响 Billing Worker 的预发布部署。</p></a><a class="alert-item" href="#/web/deployments/dep-1040"><strong>${icon("x")} Billing Worker 部署失败</strong><p>脚本缺少运行时依赖，退出状态为 127。</p></a></div></aside></div>`, "overview", "概览", "今天的部署运行状态", `<a class="btn btn--primary" href="#/web/deployments/new">${icon("deploy")} 发起部署</a>`);
  }

  function deploymentRows(items) {
    const rows=items.map(deployment=>{const app=appById(deployment.appId);const node=nodeById(deployment.nodeId);return `<tr><td><a class="cell-main" href="#/web/deployments/${deployment.id}"><span class="resource-mark">${deployment.number.replace("#","")}</span><span class="cell-stack"><strong>${app.name}</strong><span>${deployment.number} · ${deployment.environment}</span></span></a></td><td>${status(deployment)}${deploymentStatus(deployment)==="running"?`<div class="progress" style="margin-top:6px"><span style="width:${deployment.progress}%"></span></div>`:""}</td><td><div class="cell-stack"><strong>${node.name}</strong><span>${node.address}</span></div></td><td><div class="cell-stack"><strong>${deployment.version}</strong><span class="mono">${deployment.commit}</span></div></td><td>${deployment.actor}</td><td><div class="cell-stack"><strong>${deployment.createdAt}</strong><span>${deployment.duration}</span></div></td></tr>`;}).join("");
    return `<table class="data-table"><thead><tr><th>部署</th><th>状态</th><th>目标</th><th>版本</th><th>发起人</th><th>时间</th></tr></thead><tbody>${rows}</tbody></table>`;
  }

  function renderWebDeployments() {
    const data = scenarioData();
    let items = data.deployments;
    if (state.webDeploymentFilter !== "all") items = items.filter((d) => deploymentStatus(d) === state.webDeploymentFilter);
    if (state.environmentFilter !== "all") items = items.filter((d) => d.environment === state.environmentFilter);
    if (state.appFilter !== "all") items = items.filter((d) => d.appId === state.appFilter);
    if (state.nodeFilter !== "all") items = items.filter((d) => d.nodeId === state.nodeFilter);
    if (state.query) items = items.filter((d) => `${appById(d.appId).name} ${d.number} ${d.version}`.toLowerCase().includes(state.query.toLowerCase()));
    if(state.scenario==="no-results")items=[];
    const appOptions=allApps().map(app=>`<option value="${app.id}" ${state.appFilter===app.id?"selected":""}>${app.name}</option>`).join("");
    const nodeOptions=allNodes().map(node=>`<option value="${node.id}" ${state.nodeFilter===node.id?"selected":""}>${node.name}</option>`).join("");
    const environmentOptions=["开发","测试","预发布","生产"].map(value=>`<option value="${value}" ${state.environmentFilter===value?"selected":""}>${value}</option>`).join("");
    const statusFilters=[["all","全部"],["queued","排队中"],["running","运行中"],["success","成功"],["failed","失败"],["canceled","已取消"]].map(([value,label])=>`<button class="segment ${state.webDeploymentFilter===value?"is-active":""}" data-filter="${value}" aria-pressed="${state.webDeploymentFilter===value}">${label}</button>`).join("");
    const filters=`<div class="filters filters--wrap"><div class="search"><span>${icon("search")}</span><input data-action="search" value="${state.query}" placeholder="搜索应用、编号或版本" aria-label="搜索部署"></div><select class="select" data-action="app-filter" aria-label="按应用筛选"><option value="all">全部应用</option>${appOptions}</select><select class="select" data-action="node-filter" aria-label="按节点筛选"><option value="all">全部节点</option>${nodeOptions}</select><select class="select" data-action="environment-filter" aria-label="按环境筛选"><option value="all">全部环境</option>${environmentOptions}</select><div class="segmented" aria-label="部署状态">${statusFilters}</div></div>`;
    const hasFilters=state.query||state.webDeploymentFilter!=="all"||state.environmentFilter!=="all"||state.appFilter!=="all"||state.nodeFilter!=="all";
    const visible=items.slice(0,state.visibleCounts.webDeployments);
    const summary=`<div class="filter-summary" aria-live="polite"><span>显示 ${visible.length} / ${items.length} 条部署${hasFilters?" · 已应用筛选":""}</span>${hasFilters?'<button class="btn" data-action="clear-deployment-filters">清空筛选</button>':""}</div>`;
    const more=visible.length<items.length?`<button class="btn load-more" data-action="load-more" data-kind="webDeployments">加载更多</button>`:"";
    const content=data.deployments.length?`${filters}${summary}${items.length?deploymentRows(visible)+more:renderNoResults("没有匹配的部署","调整搜索词或筛选条件后重试。")}`:emptyState("deployments");
    return webShell(content, "deploy", "部署", `${data.deployments.length} 条部署记录`, `<a class="btn btn--primary" href="#/web/deployments/new">${icon("plus")} 发起部署</a>`);
  }

  function renderDeployNew(mobile = false) {
    const data = scenarioData();
    if (mobile) return renderMobileDeployNew(data);
    const deployableApps=allApps().filter(app=>app.status!=="archived"); const selectedApp=deployableApps.find(app=>app.id===state.selectedAppId)||deployableApps[0]; const targets=allTargets().filter(target=>target.appId===selectedApp.id); const selectedTarget=targets.find(target=>target.id===state.selectedTarget)||targets[0];
    const selectedNode = nodeById(selectedTarget?.nodeId||selectedApp.nodeId);
    const blocked = selectedNode.status !== "online";
    const notice=selectedTarget?.contract==="failed"
      ? `<div class="notice notice--danger">${icon("alert")} 当前目标未通过 Schema v1 契约检查，不能部署。</div>`
      : blocked?`<div class="notice notice--danger">${icon("alert")} 目标节点当前${statusLabels[selectedNode.status]}，无法确认部署。</div>`
      : `<div class="notice">${icon("alert")} 同一目标已有任务时，新部署将进入队列。</div>`;
    const appOptions=deployableApps.map(app=>`<option value="${app.id}" ${app.id===selectedApp.id?"selected":""}>${app.name}</option>`).join("");
    const targetOptions=targets.map(target=>`<option value="${target.id}" ${selectedTarget?.id===target.id?"selected":""}>${target.environment} / ${nodeById(target.nodeId).name}</option>`).join("");
    return webShell(`<div class="form-layout"><div><section class="form-section"><h2>部署内容</h2><div class="field"><label for="deploy-app">应用</label><select id="deploy-app" data-action="select-app">${appOptions}</select></div><div class="field"><label for="deploy-version">版本</label><input id="deploy-version" value="${selectedApp.id==="atlas-api"?"v2.8.4":"v1.14.0"}"></div></section><section class="form-section"><h2>部署目标</h2><div class="field"><label for="deploy-target">环境与节点</label><select id="deploy-target" data-action="select-target">${targetOptions}</select><p class="field__hint">目标脚本由应用配置维护。</p></div><div class="field"><label for="deploy-script">脚本入口</label><input id="deploy-script" class="mono" value="${selectedTarget?.script||selectedApp.script}" readonly></div><div class="field"><label for="deploy-args">受控参数</label><input id="deploy-args" class="mono" value="${selectedTarget?.args||selectedApp.args||"--environment production"}" readonly></div></section>${notice}</div><aside class="confirm-panel"><p class="eyebrow">Final check</p><h2>确认部署内容</h2><div class="confirm-row"><span>应用与环境</span><strong>${selectedApp.name} · ${selectedTarget?.environment||selectedApp.environment}</strong></div><div class="confirm-row"><span>目标节点</span><strong>${selectedNode.name} (${selectedNode.address})</strong></div><div class="confirm-row"><span>脚本</span><strong class="mono">${selectedTarget?.script||selectedApp.script}</strong></div><div class="confirm-row"><span>受控参数</span><strong class="mono">${selectedTarget?.args||selectedApp.args||"--environment production"}</strong></div><div class="confirm-row"><span>敏感引用</span><strong class="mono">••••••••</strong></div><button class="btn btn--primary" data-action="confirm-deploy" ${blocked||selectedTarget?.contract==="failed"?"disabled":""}>${icon("deploy")} 确认并部署</button></aside></div>`,"deploy","发起部署","核对目标、脚本和参数后执行",`<a class="btn" href="#/web/deployments">取消</a>`);
  }

  function renderLogPanel(deployment, mobile = false) {
    const disconnected = state.scenario === "disconnected";
    let lines = state.createdDeployment?.id === deployment.id
      ? [["刚刚", "info", "deployment accepted · waiting for target queue"]]
      : source.logs[deployment.id] || source.logs[deployment.status === "failed" ? "dep-1040" : "dep-1042"] || [];
    if(state.scenario==="long-log")lines=[...lines,...Array.from({length:28},(_,index)=>[`14:${35+Math.floor(index/6)}:${String(index%60).padStart(2,"0")}`,index%9===0?"warn":"info",`worker-${index+1} processed release artifact with a deliberately long diagnostic message that wraps safely without exposing DEPLOY_TOKEN=••••••••`])];
    const toolError=state.logToolError?`<div class="log-tool-error" role="alert">${icon("alert")} ${state.logToolError}</div>`:"";
    if (mobile) return `<div class="mobile-log-toolbar"><span>${disconnected?"连接已断开":state.logFollowing?"实时跟随":"已暂停"}</span><div><button class="icon-btn" data-action="toggle-follow" aria-label="${state.logFollowing?"暂停跟随":"继续跟随"}">${icon(state.logFollowing?"pause":"play")}</button><button class="icon-btn" data-action="copy-log" aria-label="复制日志">${icon("copy")}</button><button class="icon-btn" data-action="log-bottom" aria-label="跳到末尾">${icon("down")}</button>${disconnected?`<button class="icon-btn" data-action="reconnect-log" aria-label="重新连接">${icon("play")}</button>`:""}</div></div>${toolError}<div class="mobile-log" data-log-body>${lines.map(([time,kind,text]) => `<div class="mobile-log-line"><span class="mobile-log-time">${time}</span> <span class="log-kind--${kind}">${kind.toUpperCase()}</span> ${text}</div>`).join("")}</div>`;
    return `<section class="log-panel"><div class="log-toolbar"><div class="log-toolbar__left"><span class="log-state ${disconnected ? "log-state--off" : ""}">${disconnected ? "连接已断开" : state.logFollowing ? "实时跟随" : "已暂停跟随"}</span></div><div class="log-toolbar__right">${disconnected?`<button class="log-btn" data-action="reconnect-log" title="重新连接" aria-label="重新连接">${icon("play")}</button>`:""}<button class="log-btn ${state.logFollowing ? "is-active" : ""}" data-action="toggle-follow" title="${state.logFollowing ? "暂停跟随" : "继续跟随"}" aria-label="${state.logFollowing ? "暂停跟随" : "继续跟随"}">${icon(state.logFollowing ? "pause" : "play")}</button><button class="log-btn" data-action="copy-log" title="复制日志" aria-label="复制日志">${icon("copy")}</button><button class="log-btn" data-action="download-log" title="下载日志" aria-label="下载日志">${icon("down")}</button><button class="log-btn" data-action="log-bottom" title="跳到末尾" aria-label="跳到末尾">${icon("arrow")}</button></div></div>${toolError}<div class="log-body" data-log-body>${lines.map(([time,kind,text]) => `<div class="log-line"><span class="log-time">${time}</span><span class="log-kind log-kind--${kind}">${kind}</span><span class="log-text">${text}</span></div>`).join("")}</div></section>`;
  }

  function renderWebDeploymentDetail(id) {
    const d = deploymentById(id); const app = appById(d.appId); const node = nodeById(d.nodeId); const retrySource=state.createdDeployment?.id===d.id&&state.retrySourceId?findDeployment(state.retrySourceId):null;
    const effectiveStatus = deploymentStatus(d);
    const failure = effectiveStatus === "failed";
    const retryable = ["failed","canceled","interrupted"].includes(effectiveStatus);
    const active = ["running","queued","canceling"].includes(effectiveStatus);
    return webShell(`${failure ? `<div class="notice notice--danger">${icon("alert")} 脚本以状态 127 退出。最后有效输出：required binary 'wkhtmltopdf' was not found。</div>` : effectiveStatus === "interrupted" ? `<div class="notice notice--warning">${icon("alert")} 控制服务重启导致执行状态未知。该任务不会自动标记为失败，请先核对节点实际状态。</div>` : state.scenario === "disconnected" ? `<div class="notice">${icon("alert")} 日志连接已断开，已有输出仍然保留。部署任务可能仍在节点上运行。</div>` : ""}
      ${effectiveStatus==="queued"?`<div class="notice">${icon("pause")} 当前队列位置：第 2 位；同一目标前序任务完成后自动开始。</div>`:""}<div class="summary-strip"><div class="summary-item"><span>版本</span><strong>${d.version}</strong></div><div class="summary-item"><span>目标节点</span><strong>${node.name}</strong></div><div class="summary-item"><span>发起人</span><strong>${d.actor}</strong></div><div class="summary-item"><span>已用时间</span><strong>${d.duration}</strong></div></div>
      <div class="detail-grid"><div><div class="timeline"><div class="timeline__step is-done">已创建</div><div class="timeline__step ${effectiveStatus !== "queued" ? "is-done" : "is-current"}">节点连接</div><div class="timeline__step ${effectiveStatus === "running" ? "is-current" : ["success","failed"].includes(effectiveStatus) ? "is-done" : ""}">脚本执行</div><div class="timeline__step ${["success","failed","canceled"].includes(effectiveStatus) ? "is-current" : ""}">执行结果</div></div><div class="section-head"><div><h2>执行日志</h2><p>${failure ? "部署已结束 · 退出状态 127" : "输出内容已自动脱敏"}</p></div></div>${renderLogPanel(d)}</div>
      <aside><div class="section-head"><div><h2>执行信息</h2></div></div><dl class="inspector"><div class="inspector__row"><dt>部署编号</dt><dd>${d.number}</dd></div>${retrySource?`<div class="inspector__row"><dt>重试来源</dt><dd>${retrySource.number}</dd></div>`:""}<div class="inspector__row"><dt>应用</dt><dd><a href="#/web/apps/${app.id}">${app.name}</a></dd></div><div class="inspector__row"><dt>环境</dt><dd>${d.environment}</dd></div><div class="inspector__row"><dt>Revision</dt><dd class="mono">${d.commit}</dd></div><div class="inspector__row"><dt>脚本</dt><dd class="mono">${app.script}</dd></div><div class="inspector__row"><dt>参数</dt><dd class="mono">TOKEN=••••••••</dd></div></dl></aside></div>`, "deploy", `${app.name} ${d.number}`, `${d.environment} · ${d.createdAt}`, `${status(effectiveStatus)}${retryable?`<button class="btn btn--primary" data-action="retry-deploy" data-id="${d.id}">${icon("play")} 重试部署</button>`:""}${active ? `<button class="btn btn--danger" data-action="cancel-deploy" data-id="${d.id}">${icon("x")} 取消部署</button>` : ""}`);
  }

  function resourceTable(kind, items) {
    if (kind === "apps") return `<table class="data-table"><thead><tr><th>应用</th><th>状态</th><th>环境</th><th>部署节点</th><th>脚本入口</th><th>最近部署</th></tr></thead><tbody>${items.map((a) => `<tr><td><a class="cell-main" href="#/web/apps/${a.id}"><span class="resource-mark">${a.name.slice(0,2).toUpperCase()}</span><span class="cell-stack"><strong>${a.name}</strong><span>${a.description}</span></span></a></td><td>${status(a)}</td><td>${a.environment}</td><td>${nodeById(a.nodeId).name}</td><td class="mono">${a.script}</td><td>${a.lastDeploy}</td></tr>`).join("")}</tbody></table>`;
    return `<table class="data-table"><thead><tr><th>节点</th><th>状态</th><th>区域</th><th>应用</th><th>资源</th><th>最近检查</th></tr></thead><tbody>${items.map((n) => `<tr><td><a class="cell-main" href="#/web/nodes/${n.id}"><span class="resource-mark">${icon("node")}</span><span class="cell-stack"><strong>${n.name}</strong><span class="mono">${n.address}</span></span></a></td><td>${status(n)}</td><td>${n.region}</td><td>${n.apps} 个</td><td>${n.cpu} CPU · ${n.memory}</td><td>${n.checkedAt}</td></tr>`).join("")}</tbody></table>`;
  }

  function renderWebResources(kind) {
    const data = scenarioData(); const isApps = kind === "apps"; let items = data[kind];
    const query=state.resourceQueries[kind]||"";const selectedStatus=state.resourceStatuses[kind]||"all";
    if(query)items=items.filter(item=>`${item.name} ${item.description||item.address}`.toLowerCase().includes(query.toLowerCase()));
    if(selectedStatus!=="all")items=items.filter(item=>item.status===selectedStatus);
    if(state.scenario==="no-results")items=[];
    const statusOptions=(isApps?[["healthy","正常"],["deploying","部署中"],["error","异常"],["archived","已归档"]]:[["online","在线"],["offline","离线"],["checking","检查中"],["disabled","已停用"]]).map(([value,label])=>`<option value="${value}" ${selectedStatus===value?"selected":""}>${label}</option>`).join("");
    const filters=`<div class="filters"><div class="search">${icon("search")}<input data-action="resource-search" data-kind="${kind}" value="${query}" placeholder="搜索${isApps?"应用":"节点"}" aria-label="搜索${isApps?"应用":"节点"}"></div><select class="select" data-action="resource-status" data-kind="${kind}" aria-label="资源状态"><option value="all">全部状态</option>${statusOptions}</select></div>`;
    const content=data[kind].length?`${filters}${items.length?resourceTable(kind,items):renderNoResults("没有匹配的资源","调整搜索词或状态筛选后重试。")}`:emptyState(kind);
    return webShell(content, isApps ? "app" : "node", isApps ? "应用" : "节点", `${data[kind].length} 个${isApps ? "应用" : "受管节点"}`, isAdmin()?(isApps?`<a class="btn btn--primary" href="#/web/apps/new">${icon("plus")} 创建应用</a>`:`<a class="btn btn--primary" href="#/web/agents">${icon("agent")} 管理 Agent</a>`):"");
  }

  function agentInstallCommand(agent) {
    return `sudo env 'DEPLOY_GO_AGENT_ID=${agent.id}' 'DEPLOY_GO_AGENT_API_BASE_URL=https://deploy.example.com' 'DEPLOY_GO_AGENT_CONTROL_URL=wss://deploy.example.com/api/v1/agent/control' 'DEPLOY_GO_AGENT_MANIFEST_URL=https://deploy.example.com/api/v1/agent/download/0_1_0/manifest.json' 'DEPLOY_GO_AGENT_ENROLLMENT_TOKEN=${agentEnrollmentToken(agent)}' bash -c "curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 'https://deploy.example.com/api/v1/agent/install' | bash"`;
  }

  function agentEnrollmentToken(agent) { return `dga_enroll_${agent.id}_once`; }

  function renderWebAgents() {
    if(!isAdmin())return renderForbidden();
    const agents=allAgents();
    const boundNodeIds=new Set(agents.map(agent=>agent.nodeId));const unboundNodes=allNodes().filter(node=>!boundNodeIds.has(node.id));
    const form=state.agentCreating?`<form class="form-section" data-agent-form><h2>创建 Agent</h2><label class="field"><span>接入节点</span><select name="nodeId"><option value="">创建新节点</option>${unboundNodes.map(node=>`<option value="${node.id}">接管：${node.name}</option>`).join("")}</select></label><label class="field"><span>Agent 名称</span><input name="name" required maxlength="80" placeholder="例如：生产节点 01" autofocus></label><label class="field"><span>环境</span><select name="environment">${["开发","测试","预发布","生产"].map(value=>`<option ${value==="开发"?"selected":""}>${value}</option>`).join("")}</select></label><div class="editor-actions"><button class="btn" type="button" data-action="hide-agent-form">取消</button><button class="btn btn--primary" type="submit">创建并生成命令</button></div></form>`:"";
    const command=state.agentCommand?`<section class="form-section"><div class="section-head"><div><h2>安装命令</h2><p>${state.agentCommand.name} 当前离线，命令已包含一次性 token，30 分钟内有效，请在目标 Linux 节点执行。</p></div><button class="btn" data-action="close-agent-command">关闭</button></div><div class="public-key"><code>${agentInstallCommand(state.agentCommand)}</code><button class="icon-btn" title="复制命令" aria-label="复制命令" data-action="copy-agent-command" data-id="${state.agentCommand.id}">${icon("copy")}</button></div></section>`:"";
    const table=agents.length?`<table class="data-table"><thead><tr><th>Agent</th><th>环境</th><th>状态</th><th>版本</th><th>架构</th><th>最后在线</th></tr></thead><tbody>${agents.map(agent=>`<tr><td><a class="cell-main" href="#/web/agents/${agent.id}"><span class="resource-mark">AG</span><span class="cell-stack"><strong>${agent.name}</strong><span>${agent.hostname||"从未连接"}</span></span></a></td><td>${agent.environment||"开发"}</td><td>${status(agent)}${agent.revoked?' <span class="subtle">已撤销</span>':""}</td><td class="mono">${agent.version||"-"}</td><td>${agent.architecture||"-"}</td><td>${agent.lastSeen||"从未连接"}</td></tr>`).join("")}</tbody></table>`:renderNoResults("还没有 Agent","创建 Agent 后复制一次性安装命令到目标服务器执行。") ;
    return webShell(`${form}${command}${table}`,"agent","Agent",`${agents.length} 个协同程序`,`<button class="btn btn--primary" data-action="show-agent-form">${icon("plus")} 创建 Agent</button>`);
  }

  function renderWebAgentDetail(id) {
    if(!isAdmin())return renderForbidden();const agent=allAgents().find(item=>item.id===id);if(!agent)return renderNotFound("Agent");
    const command=state.agentCommand?.id===id?`<section class="form-section"><h2>新的安装命令</h2><p>此前尚未使用的 token 已失效，新命令已包含一次性 token，30 分钟内有效，请在目标 Linux 节点执行。</p><div class="public-key"><code>${agentInstallCommand(agent)}</code><button class="icon-btn" title="复制命令" aria-label="复制命令" data-action="copy-agent-command" data-id="${agent.id}">${icon("copy")}</button></div></section>`:"";
    return webShell(`<div class="summary-strip"><div class="summary-item"><span>状态</span>${status(agent)}</div><div class="summary-item"><span>环境</span><strong>${agent.environment||"开发"}</strong></div><div class="summary-item"><span>版本</span><strong>${agent.version||"-"}</strong></div><div class="summary-item"><span>架构</span><strong>${agent.architecture||"-"}</strong></div><div class="summary-item"><span>最后在线</span><strong>${agent.lastSeen||"从未连接"}</strong></div></div>${command}<section class="form-section"><div class="section-head"><div><h2>安装与修复</h2><p>同一身份重跑会保留有效凭证；撤销后使用新命令重新绑定。</p></div><button class="btn" data-action="generate-agent-command" data-id="${agent.id}">重新生成命令</button></div></section><section class="danger-band"><div><h2>撤销 Agent</h2><p>关闭在线连接并撤销全部 token，节点立即转为离线。</p></div><button class="btn btn--danger" data-action="revoke-agent" data-id="${agent.id}" ${agent.revoked?"disabled":""}>${agent.revoked?"已撤销":"撤销 Agent"}</button></section>`,"agent",agent.name,agent.id,`<a class="btn" href="#/web/agents">${icon("back")} Agent 列表</a>`);
  }

  function renderNoResults(title, message) {
    return `<div class="empty empty--compact"><span class="empty__icon">${icon("search")}</span><h2>${title}</h2><p>${message}</p></div>`;
  }

  function renderNotFound(scope="页面") {
    const path=routePath(); const mobile=path.startsWith("/app"); const content=`<div class="empty"><span class="empty__icon">${icon("alert")}</span><h2>${scope}不存在</h2><p>该地址无效，或对应资源已经被删除。</p><a class="btn btn--primary" href="#/${mobile?"app/overview":"web/overview"}">返回概览</a></div>`;
    const active=path.startsWith("/web/settings")?"settings":path.startsWith("/web/deployments")?"deploy":path.startsWith("/web/apps")?"app":path.startsWith("/web/nodes")?"node":"overview";
    const mobileActive=path.startsWith("/app/mine")?"profile":path.startsWith("/app/deployments")?"deploy":path.startsWith("/app/apps")?"app":path.startsWith("/app/nodes")?"node":"overview";
    return mobile?mobileShell(`<div class="mobile-page">${content}</div>`,mobileActive,"未找到",true):webShell(content,active,"未找到","无法打开请求的内容");
  }

  function renderForbidden(mobile=false) {
    const content=`<div class="empty"><span class="empty__icon">${icon("shield")}</span><h2>没有系统管理权限</h2><p>当前账号是普通用户，系统管理功能只对唯一管理员开放。</p><a class="btn btn--primary" href="#/${mobile?"app/mine":"web/overview"}">返回</a></div>`;
    return mobile?mobileShell(`<div class="mobile-page">${content}</div>`,"profile","权限不足",true):webShell(content,"overview","权限不足","当前账号不能访问该页面");
  }

  function renderScenarioState(path,type) {
    const mobile=path.startsWith("/app"); const loading=type==="loading";
    const content=loading
      ? `<div class="loading-state" aria-label="正在加载"><span></span><span></span><span></span><span></span></div>`
      : `<div class="notice notice--danger">${icon("alert")} 节点健康摘要暂时无法加载，部署记录仍可查看。<button class="btn" data-action="retry-data">重新加载</button></div><div class="section-head"><div><h2>已有部署记录</h2><p>局部失败不会清除已加载内容</p></div></div>${mobile?mobileRows(source.deployments.slice(0,2),"deployments"):deploymentRows(source.deployments.slice(0,3))}`;
    return mobile?mobileShell(`<div class="mobile-page">${content}</div>`,path.includes("deployments")?"deploy":"overview",loading?"加载中":"暂时无法加载",false):webShell(content,path.includes("deployments")?"deploy":"overview",loading?"加载中":"部分请求失败",loading?"正在获取最新状态":"已有数据不受影响");
  }

  function renderFullError() {
    return `<main class="login-page"><div class="login-panel"><span class="empty__icon">${icon("alert")}</span><div><h1>服务暂时不可用</h1><p>控制服务没有响应，当前无法读取部署状态。</p></div><button class="btn btn--primary" data-action="retry-data">重新连接</button></div>${sourceToolbar()}</main>`;
  }

  function renderWebAppForm(id=null) {
    const app=id?findApp(id):null; if(id&&!app)return renderNotFound("应用"); const title=app?"编辑应用":"创建应用";
    const check=state.contractCheckStatus==="idle"&&app?(app.id==="billing-worker"?"failed":"success"):state.contractCheckStatus;
    return webShell(`<form class="editor-page" data-app-form data-id="${app?.id||""}">
      <section class="editor-main">
        <div class="form-section"><h2>基础信息</h2><div class="field-grid">
          <div class="field"><label for="app-name">应用名称</label><input id="app-name" name="name" required value="${app?.name||""}" placeholder="Order API"></div>
          <div class="field"><label for="app-id">应用 ID</label><input id="app-id" name="id" class="mono" required ${app?"readonly":""} value="${app?.id||""}" placeholder="order-api"></div>
          <div class="field field--wide"><label for="app-description">说明</label><input id="app-description" name="description" required value="${app?.description||""}" placeholder="订单核心服务"></div>
        </div></div>
        <div class="form-section"><h2>默认部署目标</h2><div class="field-grid">
          <div class="field"><label for="app-environment">环境</label><select id="app-environment" name="environment">${["开发","测试","预发布","生产"].map(value=>`<option ${app?.environment===value?"selected":""}>${value}</option>`).join("")}</select></div>
          <div class="field"><label for="app-node">节点</label><select id="app-node" name="nodeId">${allNodes().filter(node=>node.status!=="disabled").map(node=>`<option value="${node.id}" ${app?.nodeId===node.id?"selected":""}>${node.name} · ${statusLabels[node.status]}</option>`).join("")}</select></div>
          <div class="field field--wide"><label for="app-script">脚本固定路径</label><input id="app-script" name="script" class="mono" required value="${app?.script||"/srv/apps/order-api/scripts/deploy.sh"}"></div>
          <div class="field field--wide"><label for="app-args">受控参数</label><input id="app-args" name="args" class="mono" value="${app?.args||"--environment production"}"></div>
          <div class="field"><label for="app-timeout">超时</label><select id="app-timeout" name="timeout">${["10","20","30"].map(value=>`<option value="${value}" ${String(app?.timeout||"20")===value?"selected":""}>${value} 分钟</option>`).join("")}</select></div>
          <div class="field"><label for="app-health">健康检查</label><input id="app-health" name="health" value="${app?.health||"/health"}" placeholder="/health"></div>
          <div class="field field--wide"><label for="app-secret">敏感变量引用</label><input id="app-secret" name="secretRef" class="mono" value="${app?.secretRef||"secret/prod/deploy-token"}" readonly></div>
        </div></div>
        <div class="form-section"><h2>脚本契约</h2><div class="check-panel ${check==="success"?"is-success":check==="failed"?"is-failed":""}"><div><strong>${check==="success"?"Schema v1 校验通过":check==="failed"?"Schema v1 校验失败":"等待校验"}</strong><p>${check==="success"?"事件前缀、最终状态和退出码规则有效。":check==="failed"?"缺少 deploy.result 事件、最终状态与退出码不一致，并检测到敏感输出风险。":"检查脚本事件、受控参数和敏感输出规则。"}</p></div><button class="btn" type="button" data-action="validate-contract">${icon("check")} ${check==="idle"?"校验配置":"再次校验"}</button></div></div>
      </section>
      <aside class="editor-aside"><h2>轻量脚本托管</h2><p>平台保存调用配置，不编辑脚本内容。环境变量只保存受控引用。</p><dl class="inspector"><div class="inspector__row"><dt>契约</dt><dd>Schema v1</dd></div><div class="inspector__row"><dt>互斥</dt><dd>同目标排队</dd></div><div class="inspector__row"><dt>敏感值</dt><dd>仅引用</dd></div></dl></aside>
      <div class="editor-actions"><a class="btn" href="#/web/apps">取消</a><button class="btn btn--primary" type="submit" ${check!=="success"?"disabled":""}>保存应用</button></div>
    </form>`,"app",title,app?app.description:"配置应用和首个部署目标");
  }

  function renderWebTargetForm(appId,targetId=null) {
    const app=findApp(appId); if(!app)return renderNotFound("应用"); const target=targetId?allTargets().find(item=>item.appId===appId&&item.id===targetId):null; if(targetId&&!target)return renderNotFound("部署目标");
    const check=state.targetContractCheckStatus==="idle"&&target?(target.contract==="failed"?"failed":"success"):state.targetContractCheckStatus;
    return webShell(`<form class="editor-page" data-target-form data-app-id="${app.id}" data-id="${target?.id||""}">
      <section class="editor-main"><div class="form-section"><h2>目标与脚本</h2><div class="field-grid">
        <div class="field"><label for="target-id">目标 ID</label><input id="target-id" name="id" required class="mono" ${target?"readonly":""} value="${target?.id||`${app.id}-prod`}"></div>
        <div class="field"><label for="target-environment">环境</label><select id="target-environment" name="environment">${["开发","测试","预发布","生产"].map(value=>`<option ${target?.environment===value?"selected":""}>${value}</option>`).join("")}</select></div>
        <div class="field"><label for="target-node">执行节点</label><select id="target-node" name="nodeId">${allNodes().filter(node=>node.status==="online").map(node=>`<option value="${node.id}" ${target?.nodeId===node.id?"selected":""}>${node.name}</option>`).join("")}</select></div>
        <div class="field"><label for="target-timeout">超时</label><select id="target-timeout" name="timeout">${["10","20","30"].map(value=>`<option value="${value}" ${String(target?.timeout||"20")===value?"selected":""}>${value} 分钟</option>`).join("")}</select></div>
        <div class="field field--wide"><label for="target-script">脚本固定路径</label><input id="target-script" name="script" required class="mono" value="${target?.script||app.script}"></div>
        <div class="field field--wide"><label for="target-args">受控参数</label><input id="target-args" name="args" class="mono" value="${target?.args||app.args||"--environment production --modules api"}"></div>
        <div class="field field--wide"><label for="target-secret">敏感变量引用</label><input id="target-secret" name="secretRef" class="mono" value="${target?.secretRef||app.secretRef||"secret/prod/deploy-token"}" readonly></div>
      </div></div>
      <div class="form-section"><h2>脚本契约</h2><div class="check-panel ${check==="success"?"is-success":check==="failed"?"is-failed":""}"><div><strong>${check==="success"?"Schema v1 校验通过":check==="failed"?"Schema v1 校验失败":"等待校验"}</strong><p>${check==="success"?"事件、退出码和敏感输出规则有效。":check==="failed"?"缺少最终结果事件，目标暂时不能部署。":"保存前校验该目标的脚本调用配置。"}</p></div><button class="btn" type="button" data-action="validate-target-contract">${icon("check")} ${check==="idle"?"校验目标":"再次校验"}</button></div></div>
      <div class="notice">${icon("alert")} 同一应用和目标已有任务时，新部署进入队列。</div></section>
      <aside class="editor-aside"><h2>部署后验证</h2><div class="field"><label for="target-health">HTTP 路径</label><input id="target-health" name="health" value="${target?.health||app.health||"/health"}"></div><div class="field"><label for="target-status">成功状态码</label><input id="target-status" name="successCode" value="${target?.successCode||"200"}" inputmode="numeric"></div></aside>
      <div class="editor-actions"><a class="btn" href="#/web/apps/${app.id}">取消</a><button class="btn btn--primary" type="submit" ${check!=="success"?"disabled":""}>保存目标</button></div>
    </form>`,"app",target?"编辑部署目标":"新增部署目标",`${app.name} · ${app.environment}`);
  }

  function renderLogin(mobile=false) {
    const form=`<div class="login-panel"><div class="login-brand"><span class="brand__mark">DG</span><div><strong>Deploy Go</strong><span>部署控制服务</span></div></div><div><h1>登录</h1><p>使用管理员分配的账号进入控制台。</p></div>${state.scenario==="session-expired"?`<div class="notice">${icon("alert")} 会话已失效，请重新登录。</div>`:""}${state.loginError?`<div class="notice notice--danger">${icon("alert")} 邮箱或密码不正确，请重新输入。</div>`:""}<form data-login-form><div class="field"><label for="login-email">登录邮箱</label><input id="login-email" name="email" type="email" required value="${mobile?"lin@deploy.go":"chen@deploy.go"}" autocomplete="username"></div><div class="field"><label for="login-password">密码</label><input id="login-password" name="password" type="password" required value="deploygo123" autocomplete="current-password"></div><button class="btn btn--primary" type="submit">登录</button></form><p class="login-help">账号由唯一管理员创建和分配。</p></div>`;
    if(mobile)return `<div class="app-preview"><div class="device"><div class="mobile-shell mobile-shell--secondary"><div class="mobile-status"><span>9:41</span><span>5G · 92%</span></div><main class="mobile-content mobile-login">${form}</main></div></div>${sourceToolbar()}${renderOverlay()}</div>`;
    return `<main class="login-page">${form}${sourceToolbar()}${renderOverlay()}</main>`;
  }

  function renderSetup(mobile=false) {
    const form=`<div class="login-panel"><div class="login-brand"><span class="brand__mark">DG</span><div><strong>Deploy Go</strong><span>首次初始化</span></div></div><div><h1>创建管理员</h1><p>全新实例首次访问时创建唯一管理员，完成后登录入口自动关闭。</p></div><form data-setup-form><div class="field"><label for="setup-name">管理员姓名</label><input id="setup-name" name="name" required value="陈舟"></div><div class="field"><label for="setup-email">登录邮箱</label><input id="setup-email" name="email" type="email" required value="chen@deploy.go"></div><div class="field"><label for="setup-password">初始密码</label><input id="setup-password" name="password" type="password" minlength="8" required></div><button class="btn btn--primary" type="submit">完成初始化</button></form><p class="login-help">初始化必须在系统仍为空库时完成，初始化后无法再次进入。</p></div>`;
    if(mobile)return `<div class="app-preview"><div class="device"><div class="mobile-shell mobile-shell--secondary"><div class="mobile-status"><span>9:41</span><span>5G · 92%</span></div><main class="mobile-content mobile-login">${form}</main></div></div>${sourceToolbar()}${renderOverlay()}</div>`;
    return `<main class="login-page">${form}${sourceToolbar()}${renderOverlay()}</main>`;
  }

  function renderWebUserGrants(id) {
    if(!isAdmin())return renderForbidden(); const user=allManagedUsers().find(item=>item.id===id); if(!user)return renderNotFound("用户"); const grants=grantedApps(id);
    return webShell(`<div class="section-head"><div><h2>允许访问的应用</h2><p>普通用户只能查看和部署明确授权的应用。</p></div></div><div class="grant-list">${allApps().filter(app=>app.status!=="archived").map(app=>`<label class="grant-row"><span class="cell-main"><span class="resource-mark">${app.name.slice(0,2).toUpperCase()}</span><span class="cell-stack"><strong>${app.name}</strong><span>${app.environment} · ${app.description}</span></span></span><input type="checkbox" data-grant-user="${user.id}" data-grant-app="${app.id}" ${grants.has(app.id)?"checked":""}></label>`).join("")}</div><div class="notice">${icon("shield")} 权限由 API 强制；取消授权不会删除历史部署记录。</div>`,"settings","应用授权",`${user.name} · ${user.email}`,`<a class="btn" href="#/web/settings/users/${user.id}">${icon("back")} 用户详情</a>`);
  }

  function renderWebSettings() {
    if(!isAdmin())return renderForbidden();
    const settings=state.systemSettings;
    return webShell(`<form class="settings-form" data-settings-form><div class="section-head"><div><h2>部署默认值</h2><p>应用目标未单独设置时使用</p></div></div><div class="field-grid"><div class="field"><label for="setting-concurrency">同目标并发策略</label><select id="setting-concurrency" name="concurrency"><option value="queue" ${settings.concurrency==="queue"?"selected":""}>排队</option><option value="reject" ${settings.concurrency==="reject"?"selected":""}>拒绝</option></select></div><div class="field"><label for="setting-timeout">默认执行超时</label><select id="setting-timeout" name="timeout"><option value="20" ${settings.timeout==="20"?"selected":""}>20 分钟</option><option value="30" ${settings.timeout==="30"?"selected":""}>30 分钟</option></select></div><div class="field"><label for="setting-retention">日志保留</label><select id="setting-retention" name="retention"><option value="90" ${settings.retention==="90"?"selected":""}>90 天</option><option value="180" ${settings.retention==="180"?"selected":""}>180 天</option></select></div><div class="field"><label for="setting-session">当前会话</label><input id="setting-session" value="macOS · Chrome · 当前设备" readonly></div></div><button class="btn btn--primary" type="submit">保存设置</button></form>`,"settings","系统设置","仅唯一管理员可访问");
  }

  function renderWebUsers() {
    if(!isAdmin())return renderForbidden(); const users=allManagedUsers();
    return webShell(`<table class="data-table"><thead><tr><th>用户</th><th>身份</th><th>状态</th><th>最近活动</th><th>操作</th></tr></thead><tbody>${users.map(user=>{const disabled=state.disabledUserIds.has(user.id);return `<tr><td><a class="cell-main" href="#/web/settings/users/${user.id}"><span class="avatar">${user.name.slice(0,1)}</span><span class="cell-stack"><strong>${user.name}</strong><span>${user.email}</span></span></a></td><td>${user.role}</td><td>${status(disabled?"disabled":"online")}</td><td>${user.lastActive}</td><td><a class="btn" href="#/web/settings/users/${user.id}">查看</a></td></tr>`;}).join("")}</tbody></table>`,"settings","用户管理",`${users.length} 个系统账号`,`<a class="btn btn--primary" href="#/web/settings/users/new">${icon("plus")} 新增用户</a>`);
  }

  function renderWebUserForm() {
    if(!isAdmin())return renderForbidden();
    return webShell(`<form class="form-narrow" data-web-user-create><div class="form-section"><h2>账号资料</h2><div class="field"><label for="web-user-name">姓名</label><input id="web-user-name" name="name" required></div><div class="field"><label for="web-user-email">登录邮箱</label><input id="web-user-email" name="email" type="email" required></div><div class="field"><label for="web-user-password">初始密码</label><input id="web-user-password" name="password" type="password" minlength="8" required><p class="field__hint">初始密码仅在创建时提交，不在设计源中长期保存。</p></div><div class="field"><label for="web-user-role">身份</label><input id="web-user-role" value="普通用户" readonly></div></div><div class="notice">${icon("shield")} 系统管理功能只对唯一管理员开放。</div><div class="form-actions"><a class="btn" href="#/web/settings/users">取消</a><button class="btn btn--primary" type="submit">创建用户</button></div></form>`,"settings","新增用户","管理员直接分配普通用户账号");
  }

  function renderWebUserDetail(id) {
    if(!isAdmin())return renderForbidden(); const user=allManagedUsers().find(item=>item.id===id); if(!user)return renderNotFound("用户"); const disabled=state.disabledUserIds.has(user.id);
    const grantAction=user.admin?"":`<a class="btn" href="#/web/settings/users/${user.id}/grants">${icon("shield")} 应用授权</a>`;
    return webShell(`<div class="detail-grid"><section><div class="profile-heading"><span class="avatar avatar--large">${user.name.slice(0,1)}</span><div><h2>${user.name}</h2><p>${user.email}</p></div>${status(disabled?"disabled":"online")}</div><div class="summary-strip"><div class="summary-item"><span>身份</span><strong>${user.role}</strong></div><div class="summary-item"><span>最近活动</span><strong>${user.lastActive}</strong></div><div class="summary-item"><span>应用授权</span><strong>${user.admin?"全部应用":`${grantedApps(user.id).size} 个应用`}</strong></div><div class="summary-item"><span>会话</span><strong>${user.admin?"当前 1 个":"0 个"}</strong></div></div></section><aside><div class="section-head"><h2>账号操作</h2></div>${user.admin?`<div class="notice">${icon("shield")} 唯一管理员不能停用或变更身份。</div>`:`<button class="btn ${disabled?"btn--primary":"btn--danger"}" data-action="toggle-user" data-id="${user.id}">${disabled?"启用用户":"停用用户"}</button>`}<button class="btn check-again" data-toast="初始凭证已重新分配">重新分配初始凭证</button></aside></div>`,"settings",user.name,user.email,`<a class="btn" href="#/web/settings/users">${icon("back")} 用户列表</a>${grantAction}`);
  }

  function renderWebAudit() {
    if(!isAdmin())return renderForbidden(); const rows=[...state.auditEvents,["14:32","陈舟","发起部署","Atlas API #1042","成功"],["11:48","陈舟","部署失败","Billing Worker #1040","失败"],["昨天 18:06","林臻","查看日志","Console Web #1041","成功"],["7 月 30 日","陈舟","编辑节点","sh-prod-01","成功"]];
    return webShell(`<div class="filters"><div class="search">${icon("search")}<input placeholder="搜索操作者、对象或动作"></div><select class="select"><option>全部动作</option><option>登录</option><option>配置变更</option><option>部署操作</option></select></div><table class="data-table"><thead><tr><th>时间</th><th>操作者</th><th>动作</th><th>对象</th><th>结果</th></tr></thead><tbody>${rows.map(row=>`<tr><td>${row[0]}</td><td>${row[1]}</td><td>${row[2]}</td><td>${row[3]}</td><td>${row[4]==="失败"?status("failed"):status("success")}</td></tr>`).join("")}</tbody></table>`,"settings","审计记录","登录、配置变更和部署操作");
  }

  function renderWebResourceDetail(kind, id) {
    const isApp = kind === "apps"; const item = isApp ? findApp(id) : findNode(id); if(!item)return renderNotFound(isApp?"应用":"节点");
    const linkedAgent=isApp?null:allAgents().find(agent=>agent.nodeId===item.id);
    const related = source.deployments.filter((d) => isApp ? d.appId === item.id : d.nodeId === item.id).slice(0,4);
    const targets=isApp?allTargets().filter(target=>target.appId===item.id):[];
    const content = `<div class="summary-strip"><div class="summary-item"><span>当前状态</span>${status(item)}</div><div class="summary-item"><span>${isApp ? "环境" : "区域"}</span><strong>${isApp ? item.environment : item.region}</strong></div><div class="summary-item"><span>${isApp ? "部署目标" : "承载应用"}</span><strong>${isApp ? `${targets.length} 个` : `${item.apps} 个`}</strong></div><div class="summary-item"><span>最近活动</span><strong>${isApp ? item.lastDeploy : item.checkedAt}</strong></div></div><div class="detail-grid"><section><div class="section-head"><div><h2>${isApp ? "部署目标" : "关联部署"}</h2><p>${isApp ? "脚本入口、契约和节点关系" : "该节点最近执行的任务"}</p></div>${isApp&&isAdmin()?`<a class="btn" href="#/web/apps/${item.id}/targets/new">${icon("plus")} 新增目标</a>`:""}</div>${isApp ? `<div class="activity-list">${targets.map(target=>`<div class="activity-row"><span class="activity-row__dot ${target.contract==="failed"?"activity-row__dot--danger":""}"></span><div><strong>${target.environment} / ${target.id}</strong><span class="muted">${nodeById(target.nodeId).name} · <span class="mono">${target.script}</span></span></div><div class="target-actions">${target.contract==="failed"?'<span class="status status--failed">契约失败</span>':'<span class="status status--success">契约有效</span>'}${isAdmin()?`<a class="icon-btn" title="编辑目标" aria-label="编辑目标" href="#/web/apps/${item.id}/targets/${target.id}/edit">${icon("settings")}</a>`:""}${item.status === "archived" ? status(item) : `<a class="btn" href="#/web/deployments/new">发起部署</a>`}</div></div>`).join("")}</div>` : related.length ? deploymentRows(related) : emptyState("deployments")}</section><aside><div class="section-head"><h2>基础信息</h2></div><dl class="inspector">${isApp ? `<div class="inspector__row"><dt>应用 ID</dt><dd class="mono">${item.id}</dd></div><div class="inspector__row"><dt>说明</dt><dd>${item.description}</dd></div><div class="inspector__row"><dt>脚本契约</dt><dd>Schema v1</dd></div><div class="inspector__row"><dt>互斥策略</dt><dd>同目标排队</dd></div>` : `<div class="inspector__row"><dt>节点 ID</dt><dd class="mono">${item.id}</dd></div><div class="inspector__row"><dt>地址</dt><dd class="mono">${item.address}</dd></div><div class="inspector__row"><dt>系统</dt><dd>Ubuntu 24.04 / amd64</dd></div><div class="inspector__row"><dt>Docker</dt><dd>27.1.1</dd></div><div class="inspector__row"><dt>systemd</dt><dd>可用</dd></div><div class="inspector__row"><dt>CPU</dt><dd>${item.cpu}</dd></div><div class="inspector__row"><dt>内存</dt><dd>${item.memory}</dd></div>`}</dl></aside></div>`;
    const affectedApps=isApp?[]:allApps().filter(app=>app.nodeId===item.id);
    const checkResult=state.nodeCheckResults[item.id];
    const configuration=`<section class="configuration-band"><div class="section-head"><div><h2>${isApp?"执行配置":"执行边界与影响"}</h2><p>${isApp?"脚本参数与部署后验证":"由 Agent 上报运行目录并执行受限检查"}</p></div></div><dl class="configuration-grid">${isApp
      ? `<div><dt>受控参数</dt><dd class="mono">${item.args||"--environment production"}</dd></div><div><dt>超时</dt><dd>${item.timeout||"20"} 分钟</dd></div><div><dt>健康检查</dt><dd class="mono">${item.health||"/health"}</dd></div><div><dt>敏感引用</dt><dd class="mono">${item.secretRef||"secret/prod/deploy-token"}</dd></div>`
      : `<div><dt>关联 Agent</dt><dd>${linkedAgent?`<a href="#/web/agents/${linkedAgent.id}">${linkedAgent.name}</a>`:"未关联"}</dd></div><div><dt>工作目录</dt><dd class="mono">${item.directory||"/srv/deploy"}</dd></div><div><dt>检查方式</dt><dd>SystemInspect</dd></div><div><dt>最近检查结果</dt><dd>${checkResult==="failed"?"检查失败":checkResult==="success"?"检查通过":"尚无新结果"}</dd></div><div class="configuration-grid__wide"><dt>受影响应用</dt><dd>${affectedApps.length?affectedApps.map(app=>app.name).join("、"):"无"}</dd></div>`}</dl></section>`;
    const lifecycle=isApp
      ? `<button class="btn ${item.status==="archived"?"":"btn--danger"}" data-action="toggle-app-archive" data-id="${item.id}">${icon(item.status==="archived"?"play":"pause")} ${item.status==="archived"?"恢复应用":"归档应用"}</button>`
      : `<button class="btn ${item.status==="disabled"?"":"btn--danger"}" data-action="toggle-node" data-id="${item.id}">${icon(item.status==="disabled"?"play":"pause")} ${item.status==="disabled"?"启用节点":`停用节点 · 影响 ${item.apps} 个应用`}</button>`;
    const actions=`<a class="btn" href="#/web/${kind}">${icon("back")} 返回列表</a>${isAdmin()?(isApp?`<a class="btn" href="#/web/${kind}/${item.id}/edit">${icon("settings")} 编辑</a>${lifecycle}`:linkedAgent?`<a class="btn" href="#/web/agents/${linkedAgent.id}">${icon("agent")} 查看 Agent</a>`:`<a class="btn" href="#/web/agents">${icon("agent")} 管理 Agent</a>`):""}`;
    const onboarding=isApp?"":`<section class="configuration-band"><div class="section-head"><div><h2>Agent 与节点能力</h2><p>节点身份和在线状态由 Agent 连接维护，检查通过 SystemInspect 任务执行。</p></div></div><div class="onboarding-grid"><div class="check-panel ${linkedAgent?.status==="online"?"is-success":""}"><div><strong>${linkedAgent?linkedAgent.name:"尚未关联 Agent"}</strong><p>${linkedAgent?`${linkedAgent.version||"尚未上报版本"} · ${linkedAgent.architecture||"尚未上报架构"}`:"创建 Agent 并运行一键安装命令后自动建立节点身份。"}</p></div>${linkedAgent?`<a class="btn" href="#/web/agents/${linkedAgent.id}">查看 Agent</a>`:`<a class="btn" href="#/web/agents">管理 Agent</a>`}</div><div class="check-panel"><div><strong>节点能力检查</strong><p>${linkedAgent?.status==="online"?"检查系统、架构、工作目录和可用磁盘，不执行部署脚本。":"Agent 离线或未关联，恢复在线后才能检查。"}</p></div><button class="btn" data-action="check-node" data-id="${item.id}" ${linkedAgent?.status==="online"?"":"disabled"}>${icon("check")} ${state.checkingNodeIds.has(item.id)?"检查中":"执行检查"}</button></div></div></section>`;
    return webShell(content+configuration+onboarding, isApp ? "app" : "node", item.name, isApp ? item.description : `${item.address} · ${item.region}`, actions);
  }

  function mobileNav(active) {
    return `<nav class="mobile-nav" aria-label="底部导航">${[["overview","概览","/app/overview"],["resource","资源","/app/resources"],["deploy","部署","/app/deployments"],["profile","我的","/app/mine"]].map(([i,l,path]) => `<a class="${i === active ? "is-active" : ""}" href="#${path}" ${i===active?'aria-current="page"':""}>${icon(i==="resource"?"app":i)}<span>${l}</span></a>`).join("")}</nav>`;
  }

  function mobileShell(content, active, title, secondary = false, hideHeader = false) {
    const header = hideHeader ? "" : `<header class="mobile-head">${secondary ? `<button class="icon-btn" data-action="back" aria-label="返回">${icon("back")}</button>` : ""}<h1>${title}</h1></header>`;
    return `<div class="app-preview"><div class="device"><div class="mobile-shell ${secondary ? "mobile-shell--secondary" : ""}"><div class="mobile-status"><span>9:41</span><span>5G · 92%</span></div><main class="mobile-content">${header}${content}</main>${secondary ? "" : mobileNav(active)}</div></div>${sourceToolbar()}${renderOverlay()}</div>`;
  }

  function mobileRows(items, kind) {
    if (!items.length) return emptyState(kind, true);
    return `<div class="mobile-list">${items.map((item) => {
      if (kind === "deployments") { const a=appById(item.appId); return `<a class="mobile-row" href="#/app/deployments/${item.id}"><span class="resource-mark">${item.number.replace("#","")}</span><span class="mobile-row__main"><strong>${a.name} · ${item.version}</strong><span>${nodeById(item.nodeId).name} · ${item.actor}</span></span><span class="mobile-row__tail">${status(item)}<time>${item.createdAt}</time></span></a>`; }
      const isApp = kind === "apps"; return `<a class="mobile-row" href="#/app/${kind}/${item.id}"><span class="resource-mark">${isApp ? item.name.slice(0,2).toUpperCase() : icon("node")}</span><span class="mobile-row__main"><strong>${item.name}</strong><span>${isApp ? item.description : `${item.address} · ${item.region}`}</span></span><span class="mobile-row__tail">${status(item)}<time>${isApp ? item.lastDeploy : item.checkedAt}</time></span></a>`;
    }).join("")}</div>`;
  }

  function renderMobileOverview() {
    const data=scenarioData(); if (!data.nodes.length) return mobileShell(`<div class="mobile-page">${emptyState("nodes",true)}</div>`,"overview","概览");
    return mobileShell(`<div class="mobile-page"><div class="mobile-metrics"><div class="mobile-metric"><span>运行中的部署</span><strong>${data.deployments.filter(d=>["running","queued"].includes(deploymentStatus(d))).length}</strong></div><div class="mobile-metric"><span>异常节点</span><strong>${data.nodes.filter(n=>n.status==="offline").length}</strong></div></div><section class="mobile-section"><div class="mobile-section__head"><h2>正在发生</h2><a href="#/app/deployments">查看全部</a></div>${mobileRows(data.deployments.slice(0,3),"deployments")}</section><section class="mobile-section"><div class="mobile-section__head"><h2>需要关注</h2></div><a class="alert-item" href="#/app/nodes/node-hz-01"><strong>${icon("alert")} hz-staging-01 离线</strong><p>影响 Billing Worker 的预发布部署。</p></a></section></div>`,"overview","概览");
  }

  function renderMobileMine() {
    const current=isAdmin()?allManagedUsers().find(user=>user.admin):allManagedUsers().find(user=>!user.admin);
    return mobileShell(`<section class="mine-identity mine-identity--fixed"><span class="mine-avatar">${current.name.slice(0,1)}</span><div><h2>${current.name}</h2><p>${current.email}</p></div>${status("online")}</section>
      <div class="mobile-page mine-page__body">
      <section class="settings-list mine-settings">
        ${isAdmin()?`<a class="settings-row" href="#/app/mine/users"><span class="settings-row__icon">${icon("profile")}</span><span class="settings-row__main"><strong>用户管理</strong><span>${allManagedUsers().length} 个用户 · 新增、查看与停用账号</span></span><span class="settings-row__chevron">›</span></a>`:""}
        <a class="settings-row" href="#/app/mine/profile"><span class="settings-row__icon">${icon("profile")}</span><span class="settings-row__main"><strong>个人资料</strong><span>姓名、邮箱与安全设置</span></span><span class="settings-row__chevron">›</span></a>
        <a class="settings-row" href="#/app/mine/preferences"><span class="settings-row__icon">${icon("settings")}</span><span class="settings-row__main"><strong>通知与偏好</strong><span>部署结果、异常节点与显示设置</span></span><span class="settings-row__chevron">›</span></a>
        <a class="settings-row" href="#/app/mine/about"><span class="settings-row__icon">${icon("app")}</span><span class="settings-row__main"><strong>关于 Deploy Go</strong><span>产品信息、文档与开源许可</span></span><span class="settings-row__chevron">›</span></a>
      </section>
      <button class="btn mine-signout" data-action="signout">退出登录</button>
    </div>`, "profile", "我的", false, true);
  }

  function renderMobileResources() {
    const data=scenarioData();
    return mobileShell(`<div class="mobile-page"><div class="segmented mobile-segments resource-segments" role="tablist"><a class="segment is-active" role="tab" aria-selected="true" href="#/app/apps">应用</a><a class="segment" role="tab" aria-selected="false" href="#/app/nodes">节点</a></div><section class="mobile-section"><div class="mobile-section__head"><h2>应用</h2><a href="#/app/apps">查看全部</a></div>${mobileRows(data.apps.slice(0,3),"apps")}</section><section class="mobile-section"><div class="mobile-section__head"><h2>节点</h2><a href="#/app/nodes">查看全部</a></div>${mobileRows(data.nodes.slice(0,3),"nodes")}</section></div>`,"resource","资源");
  }

  function renderMobileMineDetail(page) {
    const current=isAdmin()?allManagedUsers().find(user=>user.admin):allManagedUsers().find(user=>!user.admin);
    const pages = {
      profile: ["个人资料", `<form data-profile-form><section class="detail-group profile-summary"><span class="mine-avatar">${current.name.slice(0,1)}</span><div><h2>${current.name}</h2><p>${current.role} · 账号正常</p></div></section><section class="mobile-section"><h2>基本信息</h2><div class="mobile-form-card"><div class="field"><label for="profile-name">姓名</label><input id="profile-name" name="name" value="${current.name}" required></div><div class="field"><label for="profile-email">邮箱</label><input id="profile-email" name="email" value="${current.email}" readonly></div><button class="btn btn--primary" type="submit">保存资料</button></div></section><section class="mobile-section"><h2>安全</h2><div class="key-list"><div class="key-row"><span>登录密码</span><strong>已设置</strong></div><div class="key-row"><span>最近登录</span><strong>今天 09:12</strong></div></div></section></form>`],
      preferences: ["通知与偏好", `<section class="mobile-section mobile-section--first"><h2>部署通知</h2><div class="settings-list"><label class="toggle-row"><span><strong>部署失败</strong><small>任务失败时立即通知</small></span><input type="checkbox" data-preference="failed" ${state.preferences.failed?"checked":""}><i></i></label><label class="toggle-row"><span><strong>部署完成</strong><small>成功或取消后通知</small></span><input type="checkbox" data-preference="completed" ${state.preferences.completed?"checked":""}><i></i></label><label class="toggle-row"><span><strong>异常节点</strong><small>节点离线或检查失败时通知</small></span><input type="checkbox" data-preference="node" ${state.preferences.node?"checked":""}><i></i></label></div></section><section class="mobile-section"><h2>显示</h2><div class="key-list"><div class="key-row"><span>时间格式</span><strong>24 小时制</strong></div><div class="key-row"><span>日志跟随</span><strong>默认开启</strong></div></div></section>`],
      about: ["关于 Deploy Go", `<section class="about-hero"><span class="brand__mark">DG</span><h2>Deploy Go</h2><p>轻量级自动化部署服务</p></section><section class="mobile-section"><div class="key-list"><div class="key-row"><span>脚本契约</span><strong>Schema v1</strong></div><div class="key-row"><span>开源许可</span><strong>待确定</strong></div><div class="key-row"><span>服务状态</span><strong class="text-success">运行正常</strong></div></div></section>`],
    };
    const [title, content] = pages[page] || pages.profile;
    return mobileShell(`<div class="mobile-page">${content}</div>`, "profile", title, true);
  }

  function renderMobileUsers() {
    const users=allManagedUsers(); const enabledCount=users.filter(user=>!state.disabledUserIds.has(user.id)).length;
    return mobileShell(`<div class="mobile-page"><div class="management-summary"><div><span>全部用户</span><strong>${users.length}</strong></div><div><span>正常账号</span><strong>${enabledCount}</strong></div></div><section class="mobile-section"><div class="mobile-section__head"><h2>用户</h2><a class="compact-action" href="#/app/mine/users/new">${icon("plus")} 新增</a></div><div class="management-list">${users.map(user=>{const disabled=state.disabledUserIds.has(user.id);return `<a class="management-row" href="#/app/mine/users/${user.id}"><span class="user-avatar ${disabled?"is-disabled":""}">${user.name.slice(0,1)}</span><span><strong>${user.name}</strong><small>${user.email} · ${user.role}</small></span>${status(disabled?"disabled":"online")}</a>`;}).join("")}</div></section></div>`,"profile","用户管理",true);
  }

  function renderMobileUserDetail(id) {
    const user=allManagedUsers().find(item=>item.id===id); if(!user)return renderNotFound("用户"); const disabled=state.disabledUserIds.has(user.id);
    return mobileShell(`<div class="mobile-page"><section class="detail-group profile-summary"><span class="mine-avatar ${disabled?"is-disabled":""}">${user.name.slice(0,1)}</span><div><h2>${user.name}</h2><p>${user.email}</p></div>${status(disabled?"disabled":"online")}</section><section class="mobile-section"><h2>账号信息</h2><div class="key-list"><div class="key-row"><span>身份</span><strong>${user.role}</strong></div><div class="key-row"><span>最近活动</span><strong>${user.lastActive}</strong></div><div class="key-row"><span>权限</span><strong>${user.admin?"系统管理与全部资源":"查看资源与发起部署"}</strong></div></div></section>${user.admin?`<div class="notice">${icon("shield")} 系统只保留一个管理员。管理员账号不能在此停用或变更身份。</div>`:`<section class="mobile-section"><h2>账号操作</h2><button class="btn ${disabled?"btn--primary":"btn--danger"} user-state-action" data-action="toggle-user" data-id="${user.id}">${icon(disabled?"check":"pause")} ${disabled?"启用用户":"停用用户"}</button><p class="action-hint">${disabled?"启用后，该用户可以重新登录和执行授权操作。":"停用后，该用户将无法登录，已有部署记录不会被删除。"}</p></section>`}</div>`,"profile",user.name,true);
  }

  function renderMobileUserCreate() {
    return mobileShell(`<form class="mobile-page" data-user-create><section class="mobile-form-step"><h2>账号资料</h2><div class="field"><label for="user-name">姓名</label><input id="user-name" name="name" required placeholder="输入用户姓名"></div><div class="field"><label for="user-email">登录邮箱</label><input id="user-email" name="email" type="email" required placeholder="name@example.com"></div><div class="field"><label for="user-password">初始密码</label><input id="user-password" name="password" type="password" minlength="8" required placeholder="至少 8 位"></div></section><div class="notice">${icon("shield")} 管理员直接创建账号。新账号固定为普通用户，系统管理功能仅管理员可用。</div><div class="mobile-action"><button class="btn btn--primary" type="submit">${icon("plus")} 创建用户</button></div></form>`,"profile","新增用户",true);
  }

  function renderMobileList(kind) {
    const data=scenarioData(); let items=data[kind]; const titles={deployments:"部署",apps:"应用",nodes:"节点"}; const active=kind === "deployments" ? "deploy" : "resource";
    if(kind==="deployments"&&state.mobileDeploymentFilter!=="all")items=items.filter(d=>deploymentStatus(d)===state.mobileDeploymentFilter);
    const query=state.mobileQueries[kind]||"";
    if(query)items=items.filter(item=>{const text=kind==="deployments"?`${appById(item.appId).name} ${item.number} ${item.version}`:`${item.name} ${item.description||item.address}`;return text.toLowerCase().includes(query.toLowerCase());});
    if(state.scenario==="no-results")items=[];
    const search=`<div class="mobile-search">${icon("search")}<input data-action="mobile-search" data-kind="${kind}" value="${query}" placeholder="搜索${titles[kind]}" aria-label="搜索${titles[kind]}"></div>`;
    const countKey=`mobile${kind[0].toUpperCase()}${kind.slice(1)}`;const visible=items.slice(0,state.visibleCounts[countKey]||6);const hasFilters=query||(kind==="deployments"&&state.mobileDeploymentFilter!=="all");
    const summary=`<div class="filter-summary" aria-live="polite"><span>显示 ${visible.length} / ${items.length} 项${hasFilters?" · 已应用筛选":""}</span>${hasFilters?`<button class="btn" data-action="clear-mobile-filters" data-kind="${kind}">清空</button>`:""}</div>`;
    return mobileShell(`<div class="mobile-page">${search}${kind === "deployments" ? `<div class="segmented mobile-segments">${[["all","全部"],["running","运行中"],["failed","失败"]].map(([value,label])=>`<button class="segment ${state.mobileDeploymentFilter===value?"is-active":""}" data-filter="${value}" aria-pressed="${state.mobileDeploymentFilter===value}">${label}</button>`).join("")}</div>` : ""}${summary}${items.length?mobileRows(visible,kind):renderNoResults("没有匹配结果","调整搜索词或筛选条件。")}${visible.length<items.length?`<button class="btn load-more" data-action="load-more" data-kind="${countKey}">加载更多</button>`:""}</div>${kind === "deployments" ? `<div class="mobile-action"><a class="btn btn--primary" href="#/app/deployments/new">${icon("plus")} 发起部署</a></div>` : ""}`,active,titles[kind]);
  }

  function renderMobileDeployNew(data) {
    const deployableApps=allApps().filter(app=>app.status!=="archived"); const selected=deployableApps.find(app=>app.id===state.selectedAppId)||deployableApps[0]; const targets=allTargets().filter(target=>target.appId===selected.id); const target=targets.find(item=>item.id===state.selectedTarget)||targets[0]; const node=nodeById(target?.nodeId||selected.nodeId); const blocked=node.status!=="online"||target?.contract==="failed";
    const applicationChoices=deployableApps.map(app=>`<button class="choice ${app.id===selected.id?"is-selected":""}" data-select-mobile-app="${app.id}" aria-pressed="${app.id===selected.id}"><span><strong>${app.name}</strong><span>${app.environment} · ${app.description}</span></span>${app.id===selected.id?icon("check"):""}</button>`).join("");
    const targetChoices=targets.map(item=>`<button class="choice ${item.id===target?.id?"is-selected":""}" data-select-mobile-target="${item.id}" aria-pressed="${item.id===target?.id}"><span><strong>${item.environment} · ${nodeById(item.nodeId).name}</strong><span>${item.id} · ${item.contract==="failed"?"契约失败":"Schema v1"}</span></span>${item.id===target?.id?icon("check"):""}</button>`).join("");
    const blockingNotice=target?.contract==="failed"?`<div class="notice notice--danger">脚本契约检查失败，无法部署。</div>`:node.status!=="online"?`<div class="notice notice--danger">目标节点当前${statusLabels[node.status]}，无法部署。</div>`:"";
    return mobileShell(`<div class="mobile-page"><section class="mobile-form-step"><h2>1. 选择应用</h2>${applicationChoices}</section><section class="mobile-form-step"><h2>2. 选择部署目标</h2>${targetChoices}</section><section class="mobile-form-step"><h2>3. 核对内容</h2><div class="key-list"><div class="key-row"><span>目标节点</span><strong>${node.name} · ${node.address}</strong></div><div class="key-row"><span>版本</span><strong>v2.8.4</strong></div><div class="key-row"><span>脚本入口</span><strong class="mono">${target?.script||selected.script}</strong></div><div class="key-row"><span>受控参数</span><strong class="mono">${target?.args||selected.args||"--environment production"}</strong></div><div class="key-row"><span>敏感引用</span><strong class="mono">••••••••</strong></div></div></section>${blockingNotice}</div><div class="mobile-action"><button class="btn btn--primary" data-action="confirm-deploy" ${blocked?"disabled":""}>${icon("deploy")} 确认并部署</button></div>`,"deploy","发起部署",true);
  }

  function renderMobileDeploymentDetail(id) {
    const d=deploymentById(id); const a=appById(d.appId); const effective=deploymentStatus(d); const active=["running","queued"].includes(effective);
    const failed=effective==="failed"; const retryable=["failed","canceled","interrupted"].includes(effective); const complete=["success","failed","canceled","interrupted"].includes(effective);
    const retrySource=state.createdDeployment?.id===d.id&&state.retrySourceId?findDeployment(state.retrySourceId):null;
    const queueNotice=effective==="queued"?`<div class="notice">当前队列位置：第 2 位${retrySource?` · 重试来源 ${retrySource.number}`:""}</div>`:"";
    const action=active||retryable?`<div class="mobile-action">${retryable?`<button class="btn btn--primary" data-action="retry-deploy" data-id="${d.id}">${icon("play")} 重试部署</button>`:`<button class="btn btn--danger" data-action="cancel-deploy" data-id="${d.id}">${icon("x")} 取消部署</button>`}</div>`:"";
    const interruptedNotice=effective==="interrupted"?`<div class="notice notice--warning">${icon("alert")} 远端最终状态未知，请核对节点后再重试。</div>`:"";
    return mobileShell(`<div class="mobile-detail-hero">${status(effective)}<h2>${a.name} ${d.number}</h2><p>${d.version} · ${nodeById(d.nodeId).name}</p></div><div class="mobile-page"><div class="summary-strip" style="margin-bottom:18px"><div class="summary-item"><span>发起人</span><strong>${d.actor}</strong></div><div class="summary-item"><span>已用时间</span><strong>${d.duration}</strong></div></div>${queueNotice}${interruptedNotice}<section class="mobile-section mobile-section--first"><h2>执行阶段</h2><div class="stage-list"><div class="stage-row is-complete">${icon("check")}<span><strong>部署预检</strong><small>节点、脚本与互斥锁检查通过</small></span></div><div class="stage-row ${failed?"is-failed":complete?"is-complete":"is-running"}">${icon(failed?"x":complete?"check":"deploy")}<span><strong>执行脚本</strong><small>${failed?"脚本退出状态 127":effective==="interrupted"?"远端最终状态未知":complete?"脚本执行完成":effective==="queued"?"等待前序任务释放目标":"正在执行应用托管脚本"}</small></span></div><div class="stage-row ${complete&&!failed&&effective!=="interrupted"?"is-complete":""}">${icon(complete&&!failed&&effective!=="interrupted"?"check":"pause")}<span><strong>部署后验证</strong><small>${failed?"因脚本失败未执行":effective==="interrupted"?"等待人工核对":complete?"健康检查通过":"等待脚本完成"}</small></span></div></div></section><div class="section-head"><div><h2>执行日志</h2><p>${state.scenario==="disconnected"?"连接已断开":"可暂停、复制或跳到末尾"}</p></div></div></div>${renderLogPanel(d,true)}${action}`,"deploy",a.name,true);
  }

  function renderMobileResourceDetail(kind,id) {
    const isApp=kind==="apps"; const item=isApp?appById(id):nodeById(id); const node=isApp?nodeById(item.nodeId):item;
    const related=source.deployments.filter(d=>isApp?d.appId===item.id:d.nodeId===item.id).slice(0,2);
    const targets=isApp?allTargets().filter(target=>target.appId===item.id):[];
    const checkResult=state.nodeCheckResults[item.id];
    const details=isApp
      ? targets.map(target=>`<div class="key-row"><span>${target.environment} / ${target.id}</span><strong class="${target.contract==="failed"?"text-danger":"text-success"}">${target.contract==="failed"?"契约失败":nodeById(target.nodeId).name}</strong></div>`).join("")
      : `<div class="key-row"><span>系统</span><strong>Ubuntu 24.04 LTS</strong></div><div class="key-row"><span>架构</span><strong>linux / amd64</strong></div><div class="key-row"><span>运行能力</span><strong>Docker 27 · systemd</strong></div><div class="key-row"><span>工作目录</span><strong class="mono">${item.directory||"/var/lib/deploy-go-agent/apps"}</strong></div><div class="key-row"><span>执行通道</span><strong>Agent · SystemInspect</strong></div><div class="key-row"><span>最近检查结果</span><strong class="${checkResult==="failed"?"text-danger":"text-success"}">${checkResult==="failed"?"检查失败":checkResult==="success"?"检查通过":"等待检查"}</strong></div>`;
    const action=isApp
      ? item.status!=="archived"?`<div class="mobile-action"><a class="btn btn--primary" href="#/app/deployments/new">${icon("deploy")} 发起部署</a></div>`:""
      : item.status!=="disabled"?`<div class="mobile-action"><button class="btn btn--primary" data-action="check-node" data-id="${item.id}">${icon("check")} ${state.checkingNodeIds.has(item.id)?"检查中":"重新检查"}</button></div>`:"";
    return mobileShell(`<div class="mobile-detail-hero">${status(item)}<h2>${item.name}</h2><p>${isApp?item.description:`${item.address} · ${item.region}`}</p></div><div class="mobile-page"><section class="mobile-section mobile-section--first"><h2>${isApp?"部署目标":"节点能力"}</h2><div class="key-list">${details}</div></section><section class="mobile-section"><div class="mobile-section__head"><h2>${isApp?"最近部署":"最近活动"}</h2><span class="subtle">${isApp?item.lastDeploy:item.checkedAt}</span></div>${related.length?mobileRows(related,"deployments"):`<div class="key-list"><div class="key-row"><span>部署记录</span><strong>暂无</strong></div></div>`}</section></div>${action}`,"resource",item.name,true);
  }

  function renderOverlay() {
    const configs={
      cancel:["确认取消部署？","取消请求会发送到目标节点。脚本已经产生的变更不会自动回滚。","确认取消部署","btn--danger","cancel"],
      deploy:["确认发起部署？",state.modal?.summary||"系统将创建部署记录，并执行已配置的应用脚本。","确认并发起部署","btn--primary","deploy"],
      signout:["确认退出登录？","退出后需要重新输入管理员分配的账号和密码。","确认退出登录","btn--danger","signout"],
      discard:["放弃未保存的修改？","当前页面的输入尚未保存。放弃后无法恢复这些修改。","放弃修改并离开","btn--danger","discard"],
      lifecycle:[state.modal?.title||"确认操作？",state.modal?.message||"该操作会改变资源状态。",state.modal?.confirm||"确认操作","btn--danger","lifecycle"],
      agentCommand:["重新生成安装命令？","此前尚未使用的 enrollment token 将立即失效。","确认重新生成","btn--primary","agent-command"],
    };
    const config=state.modal?configs[state.modal.type]:null;
    const key=config?`${config[4]}:${state.modal?.id||"current"}`:"";
    const pending=key&&isPending(key);
    const closeLabel=state.modal?.type==="discard"?"继续编辑":"返回";
    const modal = config ? `<div class="modal-backdrop" role="presentation"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title" aria-describedby="modal-description" ${pending?'aria-busy="true"':""}><h2 id="modal-title">${config[0]}</h2><p id="modal-description">${config[1]}</p>${inlineTask(key)}<div class="modal__actions"><button class="btn" data-action="close-modal" ${pending?"disabled":""}>${closeLabel}</button><button class="btn ${config[3]}" data-action="complete-${config[4]}" ${pending?"disabled":""}>${pending?"处理中…":config[2]}</button></div></section></div>` : "";
    return `${modal}${state.toast ? `<div class="toast" role="status" aria-live="polite">${icon("check")} ${state.toast}</div>` : ""}`;
  }

  function render() {
    const path=routePath(); let html; const isPublic=["/entry","/spec","/web/setup","/app/setup","/web/login","/app/login"].includes(path);
    if(!state.authenticated&&!isPublic)html=renderLogin(path.startsWith("/app"));
    else if(state.scenario==="session-expired"&&!isPublic)html=renderLogin(path.startsWith("/app"));
    else if(state.scenario==="full-error"&&!isPublic)html=renderFullError();
    else if(state.scenario==="loading"&&!isPublic)html=renderScenarioState(path,"loading");
    else if(state.scenario==="partial-error"&&!isPublic)html=renderScenarioState(path,"error");
    else if(state.scenario==="unauthorized"&&!isPublic)html=renderForbidden(path.startsWith("/app"));
    else if (path==="/entry") html=renderEntry();
    else if (path==="/spec") html=renderSpec();
    else if (path==="/web/setup") html=renderSetup(false);
    else if (path==="/app/setup") html=renderSetup(true);
    else if (path==="/web/login") html=state.authenticated?renderWebOverview():renderLogin(false);
    else if (path==="/app/login") html=state.authenticated?renderMobileOverview():renderLogin(true);
    else if (path==="/web"||path==="/web/overview") html=renderWebOverview();
    else if (path==="/web/deployments") html=renderWebDeployments();
    else if (path==="/web/deployments/new") html=renderDeployNew(false);
    else if (/^\/web\/deployments\/[^/]+$/.test(path)) html=findDeployment(path.split("/").pop())?renderWebDeploymentDetail(path.split("/").pop()):renderNotFound("部署");
    else if (path==="/web/apps") html=renderWebResources("apps");
    else if (path==="/web/apps/new") html=isAdmin()?renderWebAppForm():renderForbidden();
    else if (/^\/web\/apps\/[^/]+\/edit$/.test(path)) html=isAdmin()?renderWebAppForm(path.split("/")[3]):renderForbidden();
    else if (/^\/web\/apps\/[^/]+\/targets\/new$/.test(path)) html=isAdmin()?renderWebTargetForm(path.split("/")[3]):renderForbidden();
    else if (/^\/web\/apps\/[^/]+\/targets\/[^/]+\/edit$/.test(path)) html=isAdmin()?renderWebTargetForm(path.split("/")[3],path.split("/")[5]):renderForbidden();
    else if (/^\/web\/apps\/[^/]+$/.test(path)) html=renderWebResourceDetail("apps",path.split("/").pop());
    else if (path==="/web/nodes") html=renderWebResources("nodes");
    else if (/^\/web\/nodes\/[^/]+$/.test(path)) html=renderWebResourceDetail("nodes",path.split("/").pop());
    else if (path==="/web/agents") html=renderWebAgents();
    else if (/^\/web\/agents\/[^/]+$/.test(path)) html=renderWebAgentDetail(path.split("/").pop());
    else if (path==="/web/settings") html=renderWebSettings();
    else if (path==="/web/settings/users") html=renderWebUsers();
    else if (path==="/web/settings/users/new") html=renderWebUserForm();
    else if (/^\/web\/settings\/users\/[^/]+\/grants$/.test(path)) html=renderWebUserGrants(path.split("/")[4]);
    else if (/^\/web\/settings\/users\/[^/]+$/.test(path)) html=renderWebUserDetail(path.split("/").pop());
    else if (path==="/web/settings/audit") html=renderWebAudit();
    else if (path==="/app"||path==="/app/overview") html=renderMobileOverview();
    else if (path==="/app/resources") html=renderMobileResources();
    else if (path==="/app/deployments") html=renderMobileList("deployments");
    else if (path==="/app/deployments/new") html=renderDeployNew(true);
    else if (/^\/app\/deployments\/[^/]+$/.test(path)) html=findDeployment(path.split("/").pop())?renderMobileDeploymentDetail(path.split("/").pop()):renderNotFound("部署");
    else if (path==="/app/apps") html=renderMobileList("apps");
    else if (/^\/app\/apps\/[^/]+$/.test(path)) html=findApp(path.split("/").pop())?renderMobileResourceDetail("apps",path.split("/").pop()):renderNotFound("应用");
    else if (path==="/app/nodes") html=renderMobileList("nodes");
    else if (/^\/app\/nodes\/[^/]+$/.test(path)) html=findNode(path.split("/").pop())?renderMobileResourceDetail("nodes",path.split("/").pop()):renderNotFound("节点");
    else if (path==="/app/mine") html=renderMobileMine();
    else if (path==="/app/mine/users") html=isAdmin()?renderMobileUsers():renderForbidden(true);
    else if (path==="/app/mine/users/new") html=isAdmin()?renderMobileUserCreate():renderForbidden(true);
    else if (/^\/app\/mine\/users\/[^/]+$/.test(path)) html=isAdmin()?renderMobileUserDetail(path.split("/").pop()):renderForbidden(true);
    else if (/^\/app\/mine\/(profile|preferences|about)$/.test(path)) html=renderMobileMineDetail(path.split("/").pop());
    else html=renderNotFound();
    root.innerHTML=html;
    root.querySelectorAll("form").forEach((form) => { form.noValidate = true; });
    document.title = `${root.querySelector("h1")?.textContent || "Deploy Go"} · Deploy Go`;
    if(state.modal)requestAnimationFrame(()=>root.querySelector('[role="dialog"] button:not([disabled])')?.focus());
    else if(state.focusToken){const token=state.focusToken;state.focusToken=null;restoreFocus(token);}
    if (state.logFollowing) requestAnimationFrame(()=>document.querySelectorAll("[data-log-body]").forEach(el=>{el.scrollTop=el.scrollHeight;}));
  }

  function showToast(text) { state.toast=text; render(); window.setTimeout(()=>{state.toast=null;render();},2200); }

  root.addEventListener("click", (event) => {
    const anchor=event.target.closest('a[href^="#/"]');
    if(anchor&&!state.modal&&state.dirtyForm&&state.dirtyForm.route===routePath()){
      event.preventDefault();
      requestNavigation(anchor.getAttribute("href").slice(1),anchor);
      return;
    }
    const target=event.target.closest("[data-action],[data-filter],[data-go],[data-toast],[data-select-mobile-app],[data-select-mobile-target]"); if(!target)return;
    if(target.dataset.go){go(target.dataset.go);return;}
    if(target.dataset.toast){if(target.dataset.toast==="初始凭证已重新分配"){const user=allManagedUsers().find(item=>item.id===routePath().split("/").pop());recordAudit("重新分配初始凭证",user?.email||"用户");persist();}showToast(target.dataset.toast);return;}
    if(target.dataset.filter){if(routePath().startsWith("/app"))state.mobileDeploymentFilter=target.dataset.filter;else state.webDeploymentFilter=target.dataset.filter;persist();render();return;}
    if(target.dataset.selectMobileApp){state.selectedAppId=target.dataset.selectMobileApp;state.selectedTarget=allTargets().find(item=>item.appId===state.selectedAppId)?.id||"";persist();render();return;}
    if(target.dataset.selectMobileTarget){state.selectedTarget=target.dataset.selectMobileTarget;persist();render();return;}
    const action=target.dataset.action;
    if(action==="back"){requestNavigation(appParent(routePath()),target);return;}
    if(action==="confirm-deploy"){
      const app=findApp(state.selectedAppId);const deployTarget=allTargets().find(item=>item.id===state.selectedTarget);const node=nodeById(deployTarget?.nodeId||app?.nodeId);
      state.focusToken=focusToken(target);state.modal={type:"deploy",summary:`${app?.name||"应用"} 将部署到 ${node.name}，执行 ${deployTarget?.script||app?.script||"已配置脚本"}；受控参数已核对，敏感引用 ${deployTarget?.secretRef||app?.secretRef?"1 个":"0 个"}。`};render();return;
    }
    if(action==="cancel-deploy"){state.focusToken=focusToken(target);state.modal={type:"cancel",id:target.dataset.id};render();return;}
    if(action==="close-modal"){state.modal=null;state.pendingNavigation=null;render();return;}
    if(action==="signout"){state.focusToken=focusToken(target);state.modal={type:"signout"};render();return;}
    if(action==="complete-discard"){const destination=state.pendingNavigation;clearDirty();state.modal=null;if(destination)go(destination);else render();return;}
    if(action==="complete-signout"){
      const mobile=routePath().startsWith("/app");
      runTask("signout:current",()=>{state.authenticated=false;state.modal=null;clearDirty();},{toast:"已退出登录",after:()=>go(mobile?"/app/login":"/web/login")});return;
    }
    if(action==="complete-deploy"){
      const mobile=routePath().startsWith("/app");
      runTask("deploy:current",()=>{const deployableApps=allApps().filter(app=>app.status!=="archived");const app=deployableApps.find(item=>item.id===state.selectedAppId)||deployableApps[0];const targets=allTargets().filter(item=>item.appId===app.id);const deployTarget=targets.find(item=>item.id===state.selectedTarget)||targets[0];if(!app||!deployTarget||deployTarget.contract==="failed"||nodeById(deployTarget.nodeId).status!=="online")throw new Error("部署条件已变化");state.selectedAppId=app.id;state.selectedTarget=deployTarget.id;state.createdDeployment={id:`dep-${Date.now()}`,appId:app.id,number:"#1043",status:"queued",environment:deployTarget.environment,nodeId:deployTarget.nodeId,actor:isAdmin()?"陈舟":"林臻",createdAt:"刚刚",duration:"--",version:app.id==="atlas-api"?"v2.8.4":"v1.14.0",commit:"pending",progress:0};state.modal=null;recordAudit("发起部署",`${app.name} #1043`);},{toast:"部署 #1043 已进入队列",after:()=>go(`${mobile?"/app":"/web"}/deployments/${state.createdDeployment.id}`)});return;
    }
    if(action==="complete-cancel"){
      const id=state.modal.id;const deployment=findDeployment(id);
      runTask(`cancel:${id}`,()=>{state.cancelingIds.add(id);state.modal=null;recordAudit("请求取消部署",deployment?.number||id);window.setTimeout(()=>{state.cancelingIds.delete(id);state.canceledIds.add(id);recordAudit("取消部署",deployment?.number||id);persist();showToast("部署已取消");},360);},{toast:"取消请求已发送"});return;
    }
    if(action==="retry-deploy"){
      const previous=findDeployment(target.dataset.id); if(!previous)return;
      const mobile=routePath().startsWith("/app");runTask(`retry:${previous.id}`,()=>{state.retrySourceId=previous.id; state.createdDeployment={...previous,id:`dep-retry-${Date.now()}`,number:"#1043",status:"queued",createdAt:"刚刚",duration:"--",progress:0};recordAudit("重试部署",`${previous.number} → #1043`);},{toast:"重试任务已进入队列",after:()=>go(`${mobile?"/app":"/web"}/deployments/${state.createdDeployment.id}`)});return;
    }
    if(action==="toggle-user"){
      if(target.dataset.id==="chen-zhou"){showToast("唯一管理员不能停用");return;}
      const disabled=state.disabledUserIds.has(target.dataset.id);
      if(!disabled){const user=allManagedUsers().find(item=>item.id===target.dataset.id);state.focusToken=focusToken(target);state.modal={type:"lifecycle",kind:"user",id:target.dataset.id,title:`停用 ${user?.name}？`,message:"该用户将立即无法登录，已有会话会失效；历史部署记录保留。",confirm:"确认停用用户"};render();return;}
      runTask(`lifecycle:${target.dataset.id}`,()=>{state.disabledUserIds.delete(target.dataset.id);recordAudit("启用用户",allManagedUsers().find(user=>user.id===target.dataset.id)?.email||target.dataset.id);},{toast:"用户已启用"});return;
    }
    if(action==="toggle-node"){const node=findNode(target.dataset.id);if(!node)return;const enabling=node.status==="disabled";if(!enabling){const affected=allApps().filter(app=>app.nodeId===node.id);state.focusToken=focusToken(target);state.modal={type:"lifecycle",kind:"node",id:node.id,title:`停用 ${node.name}？`,message:`该节点将不能接收新部署，影响 ${affected.length} 个应用：${affected.map(app=>app.name).join("、")||"无"}。`,confirm:"确认停用节点"};render();return;}runTask(`lifecycle:${node.id}`,()=>{upsertById(state.createdNodes,{...node,status:"online",checkedAt:"刚刚"});recordAudit("启用节点",node.name);},{toast:"节点已启用"});return;}
    if(action==="show-agent-form"){state.agentCreating=true;render();return;}
    if(action==="hide-agent-form"){state.agentCreating=false;render();return;}
    if(action==="close-agent-command"){state.agentCommand=null;render();return;}
    if(action==="copy-agent-command"){const agent=allAgents().find(item=>item.id===target.dataset.id);if(!agent)return;navigator.clipboard?.writeText(agentInstallCommand(agent)).then(()=>showToast("安装命令已复制")).catch(()=>showToast("无法自动复制，请手动选择命令"));return;}
    if(action==="generate-agent-command"){state.modal={type:"agentCommand",id:target.dataset.id};render();return;}
    if(action==="complete-agent-command"){const agent=allAgents().find(item=>item.id===state.modal.id);state.agentCommand=agent;state.modal=null;recordAudit("重新生成 Agent 安装命令",agent.name);persist();showToast("新的安装命令已生成");return;}
    if(action==="revoke-agent"){const agent=allAgents().find(item=>item.id===target.dataset.id);state.modal={type:"lifecycle",kind:"agent",id:agent.id,title:`撤销 ${agent.name}？`,message:"在线连接会立即关闭，恢复时必须使用新命令重新绑定。",confirm:"确认撤销 Agent"};render();return;}
    if(action==="toggle-app-archive"){const app=findApp(target.dataset.id);if(!app)return;const restoring=app.status==="archived";if(!restoring){state.focusToken=focusToken(target);state.modal={type:"lifecycle",kind:"app",id:app.id,title:`归档 ${app.name}？`,message:"归档后不能发起新部署，现有目标和历史记录仍然保留。",confirm:"确认归档应用"};render();return;}runTask(`lifecycle:${app.id}`,()=>{upsertById(state.createdApps,{...app,status:"healthy"});recordAudit("恢复应用",app.name);},{toast:"应用已恢复"});return;}
    if(action==="complete-lifecycle"){
      const modal={...state.modal};runTask(`lifecycle:${modal.id}`,()=>{if(modal.kind==="user"){state.disabledUserIds.add(modal.id);recordAudit("停用用户",allManagedUsers().find(user=>user.id===modal.id)?.email||modal.id);}if(modal.kind==="node"){const node=findNode(modal.id);upsertById(state.createdNodes,{...node,status:"disabled",checkedAt:"刚刚"});recordAudit("停用节点",node.name);}if(modal.kind==="app"){const app=findApp(modal.id);upsertById(state.createdApps,{...app,status:"archived"});recordAudit("归档应用",app.name);}if(modal.kind==="agent"){state.revokedAgentIds.add(modal.id);if(state.agentCommand?.id===modal.id)state.agentCommand=null;recordAudit("撤销 Agent",allAgents().find(agent=>agent.id===modal.id)?.name||modal.id);}state.modal=null;},{toast:modal.kind==="user"?"用户已停用":modal.kind==="node"?"节点已停用":modal.kind==="agent"?"Agent 已撤销":"应用已归档"});return;
    }
    if(action==="toggle-follow"){state.logFollowing=!state.logFollowing;render();return;}
    if(action==="clear-deployment-filters"){state.query="";state.webDeploymentFilter="all";state.environmentFilter="all";state.appFilter="all";state.nodeFilter="all";state.visibleCounts.webDeployments=8;persist();render();return;}
    if(action==="clear-mobile-filters"){state.mobileQueries[target.dataset.kind]="";if(target.dataset.kind==="deployments")state.mobileDeploymentFilter="all";persist();render();return;}
    if(action==="load-more"){state.visibleCounts[target.dataset.kind]=(state.visibleCounts[target.dataset.kind]||6)+6;render();return;}
    if(action==="reconnect-log"){if(state.scenario==="tool-failed"){state.logToolError="重新连接失败，已加载日志仍然保留。";render();return;}state.scenario="running";state.logToolError="";persist();showToast("日志连接已恢复");return;}
    if(action==="retry-data"){state.scenario="running";persist();showToast("数据已重新加载");return;}
    if(action==="validate-contract"||action==="validate-target-contract"){
      const failed=state.scenario==="contract-failed";const stateKey=action==="validate-contract"?"contractCheckStatus":"targetContractCheckStatus";state[stateKey]=failed?"failed":"success";const panel=target.closest(".check-panel");panel?.classList.toggle("is-success",!failed);panel?.classList.toggle("is-failed",failed);const copy=panel?.querySelector("div");if(copy)copy.innerHTML=failed?"<strong>Schema v1 校验失败</strong><p>缺少 deploy.result 事件、最终状态与退出码不一致，并检测到敏感输出风险。</p>":"<strong>Schema v1 校验通过</strong><p>事件、退出码和敏感输出规则有效。</p>";target.innerHTML=`${icon("check")} 再次校验`;const submit=target.closest("form")?.querySelector('button[type="submit"]');if(submit)submit.disabled=failed;return;
    }
    if(action==="check-node"){state.checkingNodeIds.add(target.dataset.id);render();window.setTimeout(()=>{const result=state.scenario==="failed"?"failed":"success";state.checkingNodeIds.delete(target.dataset.id);state.nodeCheckResults[target.dataset.id]=result;persist();showToast(result==="failed"?"节点能力检查失败":"节点能力检查通过");},500);return;}
    if(action==="log-bottom"){state.logFollowing=true;state.logToolError="";document.querySelectorAll("[data-log-body]").forEach(el=>{el.scrollTop=el.scrollHeight;});render();return;}
    if(action==="copy-log"){if(state.scenario==="tool-failed"||!navigator.clipboard?.writeText){state.logToolError="无法复制日志，请检查剪贴板权限。";render();return;}navigator.clipboard.writeText(document.querySelector("[data-log-body]")?.innerText||"").then(()=>{state.logToolError="";showToast("日志已复制");}).catch(()=>{state.logToolError="无法复制日志，请检查剪贴板权限。";render();});return;}
    if(action==="download-log"){if(state.scenario==="tool-failed"){state.logToolError="日志下载未能发起，请重试。";render();return;}const blob=new Blob([document.querySelector("[data-log-body]")?.innerText||""],{type:"text/plain"});const link=document.createElement("a");link.href=URL.createObjectURL(blob);link.download="deployment.log";link.click();URL.revokeObjectURL(link.href);state.logToolError="";showToast("已发起日志下载");}
  });

  root.addEventListener("change", (event) => {
    const target=event.target;
    if(target.dataset.action==="scenario"){state.scenario=target.value;state.contractCheckStatus="idle";state.targetContractCheckStatus="idle";state.logFollowing=true;persist();render();}
    if(target.dataset.action==="role"){state.role=target.value;persist();render();}
    if(target.dataset.action==="select-app"){state.selectedAppId=target.value;state.selectedTarget=allTargets().find(item=>item.appId===state.selectedAppId)?.id||"";persist();render();}
    if(target.dataset.action==="select-target"){state.selectedTarget=target.value;persist();render();}
    if(target.dataset.action==="resource-status"){state.resourceStatuses[target.dataset.kind]=target.value;persist();render();}
    if(target.dataset.action==="app-filter"){state.appFilter=target.value;persist();render();}
    if(target.dataset.action==="node-filter"){state.nodeFilter=target.value;persist();render();}
    if(target.dataset.grantUser){const grants=grantedApps(target.dataset.grantUser);target.checked?grants.add(target.dataset.grantApp):grants.delete(target.dataset.grantApp);state.applicationGrants[target.dataset.grantUser]=[...grants];recordAudit(target.checked?"分配应用授权":"撤销应用授权",`${target.dataset.grantUser} / ${target.dataset.grantApp}`);persist();render();}
    if(target.dataset.action==="environment-filter"){state.environmentFilter=target.value;persist();render();}
    if(target.dataset.preference){state.preferences[target.dataset.preference]=target.checked;persist();showToast("通知偏好已保存");}
  });
  root.addEventListener("submit", (event) => {
    const element=event.target; event.preventDefault(); if(!validateForm(element))return; const form=new FormData(element);
    if(element.matches("[data-setup-form]")){const password=String(form.get("password")||"");if(password.length<8){showToast("初始密码至少需要 8 位");return;}state.setupComplete=true;state.authenticated=false;recordAudit("完成首次初始化","唯一管理员");persist();element.reset();go(routePath().startsWith("/app")?"/app/login":"/web/login");return;}
    if(element.matches("[data-login-form]")){event.preventDefault();const email=String(form.get("email")||"").trim();const user=allManagedUsers().find(item=>item.email===email);if(!user||state.disabledUserIds.has(user.id)||String(form.get("password")||"").length<8){state.loginError=true;render();return;}state.authenticated=true;state.loginError=false;state.role=user.admin?"admin":"user";if(state.scenario==="session-expired")state.scenario="running";recordAudit("登录",email);persist();go(routePath().startsWith("/app")?"/app/overview":"/web/overview");return;}
    if(element.matches("[data-user-create],[data-web-user-create]")){event.preventDefault();const name=String(form.get("name")||"").trim();const email=String(form.get("email")||"").trim();state.createdUsers.push({id:`user-${Date.now()}`,name,email,role:"普通用户",lastActive:"尚未登录"});recordAudit("创建用户",email);clearDirty();persist();state.toast="普通用户账号已创建";go(element.matches("[data-user-create]")?"/app/mine/users":"/web/settings/users");return;}
    if(element.matches("[data-agent-form]")){const id=`agent-${Date.now()}`;const existingNodeId=String(form.get("nodeId")||"");const existingNode=existingNodeId?findNode(existingNodeId):null;const agent={id,nodeId:existingNodeId||`node-${Date.now()}`,name:existingNode?.name||String(form.get("name")||"").trim(),environment:String(form.get("environment")||"开发"),status:"offline",version:null,hostname:null,architecture:null,lastSeen:"从未连接"};state.createdAgents.push(agent);state.agentCommand=agent;state.agentCreating=false;recordAudit(existingNode?"接管节点 Agent":"创建 Agent",agent.name);clearDirty();persist();showToast(existingNode?"历史节点已关联 Agent":"Agent 已创建，等待节点连接");render();return;}
    if(element.matches("[data-app-form]")){event.preventDefault();const existing=findApp(element.dataset.id);const app={id:String(form.get("id")),name:String(form.get("name")),description:String(form.get("description")),status:existing?.status||"healthy",environment:String(form.get("environment")),target:existing?.target||`${form.get("id")}-default`,nodeId:String(form.get("nodeId")),script:String(form.get("script")),args:String(form.get("args")),secretRef:String(form.get("secretRef")),timeout:String(form.get("timeout")),health:String(form.get("health")),lastDeploy:existing?.lastDeploy||"尚未部署"};upsertById(state.createdApps,app);state.contractCheckStatus="idle";recordAudit(existing?"编辑应用":"创建应用",app.name);clearDirty();persist();state.toast="应用配置已保存";go(`/web/apps/${app.id}`);return;}
    if(element.matches("[data-target-form]")){event.preventDefault();const target={id:String(form.get("id")),appId:element.dataset.appId,environment:String(form.get("environment")),nodeId:String(form.get("nodeId")),script:String(form.get("script")),args:String(form.get("args")),secretRef:String(form.get("secretRef")),timeout:String(form.get("timeout")),health:String(form.get("health")),successCode:String(form.get("successCode")),contract:"valid"};const existing=state.createdTargets.find(item=>item.appId===target.appId&&item.id===target.id);existing?Object.assign(existing,target):state.createdTargets.push(target);state.targetContractCheckStatus="idle";recordAudit(element.dataset.id?"编辑部署目标":"新增部署目标",`${target.appId}/${target.id}`);clearDirty();persist();state.toast="部署目标已保存";go(`/web/apps/${element.dataset.appId}`);return;}
    if(element.matches("[data-profile-form]")){event.preventDefault();const current=isAdmin()?allManagedUsers().find(user=>user.admin):allManagedUsers().find(user=>!user.admin);upsertById(state.userOverrides,{...current,name:String(form.get("name"))});recordAudit("修改个人资料",current.email);clearDirty();persist();showToast("个人资料已保存");return;}
    if(element.matches("[data-settings-form]")){event.preventDefault();state.systemSettings={concurrency:String(form.get("concurrency")),timeout:String(form.get("timeout")),retention:String(form.get("retention"))};recordAudit("修改系统设置","部署默认值");clearDirty();persist();showToast("系统设置已保存");return;}
  });
  root.addEventListener("input", (event) => {
    const action=event.target.dataset.action;
    if(action==="search"||action==="resource-search"||action==="mobile-search"){
      if(action==="search")state.query=event.target.value;
      if(action==="resource-search")state.resourceQueries[event.target.dataset.kind]=event.target.value;
      if(action==="mobile-search")state.mobileQueries[event.target.dataset.kind]=event.target.value;
      persist();render();
      const input=document.querySelector(`[data-action="${action}"][data-kind="${event.target.dataset.kind||""}"]`)||document.querySelector(`[data-action="${action}"]`);
      input?.focus();input?.setSelectionRange(input.value.length,input.value.length);
      return;
    }
    const form=event.target.closest("form");
    if(form){markDirty(form);invalidateDependentCheck(form);}
  });
  root.addEventListener("keydown",(event)=>{
    if(event.key==="Escape"&&state.modal&&!isPending(`${state.modal.type}:${state.modal.id||"current"}`)){state.modal=null;state.pendingNavigation=null;render();return;}
    if(event.key!=="Tab"||!state.modal)return;
    const controls=[...root.querySelectorAll('[role="dialog"] button:not([disabled])')];if(!controls.length)return;
    const first=controls[0];const last=controls[controls.length-1];
    if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}
    else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
  });
  root.addEventListener("scroll",(event)=>{const body=event.target.closest?.("[data-log-body]");if(!body)return;const atBottom=body.scrollHeight-body.scrollTop-body.clientHeight<12;if(!atBottom&&state.logFollowing){state.logFollowing=false;const label=root.querySelector(".log-state, .mobile-log-toolbar > span");if(label)label.textContent="已暂停跟随";}},{capture:true});
  window.addEventListener("hashchange",()=>{
    const destination=routePath();
    if(state.navigationBypass){state.navigationBypass=false;state.currentRoute=destination;}
    else if(state.dirtyForm?.route===state.currentRoute&&destination!==state.currentRoute){
      state.pendingNavigation=destination;
      window.history.replaceState(null,"",`#${state.currentRoute}`);
      state.modal={type:"discard"};
      render();
      return;
    } else state.currentRoute=destination;
    state.nodeTestStatus="idle";state.contractCheckStatus="idle";state.targetContractCheckStatus="idle";render();
    requestAnimationFrame(()=>{const main=root.querySelector("main");if(main){main.tabIndex=-1;main.focus();}});
  });
  window.addEventListener("beforeunload",(event)=>{if(state.dirtyForm){event.preventDefault();event.returnValue="";}});
  render();
  requestAnimationFrame(()=>{const main=root.querySelector("main");if(main){main.tabIndex=-1;main.focus();}});
})();
