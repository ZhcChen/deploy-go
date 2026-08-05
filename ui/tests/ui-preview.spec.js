// Playwright 规格文件；由后续仓库级 E2E 工具配置加载。
const { test, expect } = require("@playwright/test");

const baseURL = process.env.UI_BASE_URL || "http://127.0.0.1:30102";
const webRoutes = ["overview", "deployments", "deployments/new", "deployments/dep-1040", "apps", "apps/new", "apps/atlas-api", "apps/atlas-api/edit", "apps/atlas-api/targets/new", "apps/atlas-api/targets/prod-cn-1/edit", "nodes", "nodes/node-sh-01", "agents", "agents/agent-sh-01", "settings", "settings/users", "settings/users/new", "settings/users/lin-zhen", "settings/users/lin-zhen/grants", "settings/audit"];
const appRoutes = ["overview", "resources", "deployments", "deployments/new", "deployments/dep-1042", "apps", "apps/atlas-api", "nodes", "nodes/node-sh-01", "mine", "mine/users", "mine/users/new", "mine/users/lin-zhen", "mine/profile", "mine/preferences", "mine/about"];

async function setScenario(page, scenario, route) {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="scenario"]').selectOption(scenario);
  await page.goto(`${baseURL}/#${route}`);
}

test("Web P0 路由可访问", async ({ page }) => {
  for (const route of webRoutes) {
    await page.goto(`${baseURL}/#/web/${route}`);
    await expect(page.locator("main")).toBeVisible();
    await expect(page.locator(".web-shell")).toBeVisible();
    await expect(page.getByText("不存在")).toHaveCount(0);
  }
});

test("设计源 Agent 创建、命令与撤销形成闭环", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/agents`);
  await page.getByRole("button", { name: "创建 Agent" }).click();
  await page.getByLabel("Agent 名称").fill("边缘节点 01");
  await page.getByRole("button", { name: "创建并生成命令" }).click();
  await expect(page.getByText(/边缘节点 01 当前离线/)).toBeVisible();
  const stored = await page.evaluate(() => localStorage.getItem("deploy-go-ui/design-source-v1"));
  expect(stored).not.toContain("dga_enroll_");
  await page.getByRole("link", { name: /边缘节点 01/ }).click();
  await page.getByRole("button", { name: "撤销 Agent" }).click();
  await expect(page.getByRole("dialog")).toContainText("撤销 边缘节点 01");
  await page.getByRole("button", { name: "确认撤销 Agent" }).click();
  await expect(page.getByRole("button", { name: "已撤销" })).toBeDisabled();
  await expect(page.getByRole("heading", { name: "新的安装命令" })).toHaveCount(0);
});

test("App P0 路由可访问且无横向溢出", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const route of appRoutes) {
    await page.goto(`${baseURL}/#/app/${route}`);
    await expect(page.locator(".mobile-shell")).toBeVisible();
    expect(await page.locator(".mobile-content").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
  }
});

test("App 一级导航层级符合移动端规范", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${baseURL}/#/app/overview`);
  await expect(page.locator(".mobile-head p, .mobile-head .avatar")).toHaveCount(0);
  await expect(page.locator(".mobile-nav a")).toHaveCount(4);
  await expect(page.locator(".mobile-nav")).toContainText("资源");
  await expect(page.locator(".mobile-nav a.is-active .icon")).toHaveCSS("background-color", "rgb(13, 17, 23)");
  await page.goto(`${baseURL}/#/app/mine`);
  await expect(page.locator(".mobile-head")).toHaveCount(0);
  await expect(page.locator(".mine-identity")).toBeVisible();
  await expect(page.getByText("权限说明")).toHaveCount(0);
  await page.goto(`${baseURL}/#/app/mine/role`);
  await expect(page.getByRole("heading", { name: "页面不存在" })).toBeVisible();
});

test("Web 设置使用常驻二级菜单", async ({ page }) => {
  for (const [route, label] of [["/web/settings", "系统设置"], ["/web/settings/users", "用户管理"], ["/web/settings/audit", "审计记录"]]) {
    await page.goto(`${baseURL}/#${route}`);
    await expect(page.getByRole("navigation", { name: "设置导航" }).getByRole("link")).toHaveCount(3);
    await expect(page.getByRole("navigation", { name: "设置导航" }).getByRole("link", { name: label })).toHaveAttribute("aria-current", "page");
  }
  await page.goto(`${baseURL}/#/web/settings`);
  await expect(page.locator(".settings-tile")).toHaveCount(0);
  await page.goto(`${baseURL}/#/web/settings/users`);
  await expect(page.locator(".page-head__actions").getByRole("link", { name: /设置$/ })).toHaveCount(0);
});

test("未知资源进入未找到页面", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/apps/not-exists`);
  await expect(page.getByRole("heading", { name: "应用不存在" })).toBeVisible();
  await page.goto(`${baseURL}/#/app/nodes/not-exists`);
  await expect(page.getByRole("heading", { name: "节点不存在" })).toBeVisible();
  await page.goto(`${baseURL}/#/app/mine/users/not-exists`);
  await expect(page.getByRole("heading", { name: "用户不存在" })).toBeVisible();
});

test("编辑部署目标不会创建重复目标", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="role"]').selectOption("admin");
  await page.goto(`${baseURL}/#/web/apps/atlas-api/targets/prod-cn-1/edit`);
  await page.getByLabel("脚本固定路径").fill("/srv/atlas/scripts/release.sh");
  await page.getByRole("button", { name: "保存目标" }).click();
  await expect(page.locator(".activity-row")).toHaveCount(1);
  await expect(page.getByText("/srv/atlas/scripts/release.sh")).toBeVisible();
});

test("普通用户不能进入系统管理", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="role"]').selectOption("user");
  await page.goto(`${baseURL}/#/web/settings`);
  await expect(page.getByRole("heading", { name: "没有系统管理权限" })).toBeVisible();
  await page.goto(`${baseURL}/#/app/mine`);
  await expect(page.getByText("用户管理")).toHaveCount(0);
});

test("Web 应用配置可提交", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="role"]').selectOption("admin");
  await page.goto(`${baseURL}/#/web/apps/new`);
  await page.getByLabel("应用名称").fill("Order API");
  await page.getByLabel("应用 ID").fill("order-api");
  await page.getByLabel("说明").fill("订单核心服务");
  await page.getByRole("button", { name: "校验配置" }).click();
  await expect(page.getByText("Schema v1 校验通过")).toBeVisible();
  await page.getByRole("button", { name: "保存应用" }).click();
  await expect(page.getByRole("heading", { name: "Order API" })).toBeVisible();
});

test("管理员创建普通用户并可停用", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/settings/users/new`);
  await page.getByLabel("姓名").fill("测试用户");
  await page.getByLabel("登录邮箱").fill("test@example.com");
  await page.getByLabel("初始密码").fill("initial-pass-123");
  await page.getByRole("button", { name: "创建用户" }).click();
  await page.getByRole("link", { name: /测试用户/ }).click();
  await page.getByRole("button", { name: "停用用户" }).click();
  await page.getByRole("button", { name: "确认停用用户" }).click();
  await expect(page.locator(".status", { hasText: "已停用" })).toBeVisible();
  await page.reload();
  await expect(page.locator(".status", { hasText: "已停用" })).toBeVisible();
});

test("首次 setup 后进入登录且输入不持久化", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/setup`);
  await page.getByLabel("初始密码").fill("initial-admin-pass");
  await page.getByRole("button", { name: "完成初始化" }).click();
  await expect(page).toHaveURL(/#\/web\/login$/);
  const stored = await page.evaluate(() => localStorage.getItem("deploy-go-ui/design-source-v1"));
  expect(stored).not.toContain("initial-admin-pass");
});

test("管理员可以分配和撤销普通用户应用授权", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/settings/users/lin-zhen/grants`);
  const billing = page.getByRole("checkbox").nth(2);
  await expect(billing).not.toBeChecked();
  await billing.check();
  await page.reload();
  await expect(page.getByRole("checkbox").nth(2)).toBeChecked();
  await page.getByRole("checkbox").nth(2).uncheck();
  await page.reload();
  await expect(page.getByRole("checkbox").nth(2)).not.toBeChecked();
});

test("发起部署后进入排队详情", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/deployments/new`);
  await page.getByRole("button", { name: "确认并部署" }).click();
  await page.getByRole("button", { name: "确认并发起部署" }).click();
  await expect(page).toHaveURL(/deployments\/dep-\d+/);
  await expect(page.getByText("排队中")).toBeVisible();
  await expect(page.getByText("当前队列位置：第 2 位", { exact: false })).toBeVisible();
});

test("取消部署需要二次确认", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/deployments/dep-1042`);
  await page.getByRole("button", { name: "取消部署" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("dialog").getByRole("button", { name: "返回" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "取消部署" }).click();
  await page.getByRole("button", { name: "确认取消" }).click();
  await expect(page.locator(".status", { hasText: "已取消" })).toBeVisible();
  await page.reload();
  await expect(page.locator(".status", { hasText: "已取消" })).toBeVisible();
});

test("应用生命周期在刷新后保持", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/apps/atlas-api`);
  await page.getByRole("button", { name: "归档应用" }).click();
  await page.getByRole("button", { name: "确认归档应用" }).click();
  await expect(page.locator(".status", { hasText: "已归档" }).first()).toBeVisible();
  await page.reload();
  await expect(page.getByText("已归档").first()).toBeVisible();
  await expect(page.getByRole("link", { name: "发起部署" })).toHaveCount(0);
});

test("契约失败会阻断保存", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="scenario"]').selectOption("contract-failed");
  await page.goto(`${baseURL}/#/web/apps/new`);
  await page.getByLabel("应用名称").fill("Invalid App");
  await page.getByLabel("应用 ID").fill("invalid-app");
  await page.getByLabel("说明").fill("契约失败示例");
  await page.getByRole("button", { name: "校验配置" }).click();
  await expect(page.getByText("最终状态与退出码不一致", { exact: false })).toBeVisible();
  await expect(page.getByRole("button", { name: "保存应用" })).toBeDisabled();
});

test("密集历史与局部失败场景仍可继续操作", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="scenario"]').selectOption("dense");
  await page.goto(`${baseURL}/#/web/deployments`);
  await page.locator("tbody a").first().click();
  await expect(page.getByText("部署不存在")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: /#/ })).toBeVisible();

  await page.locator('[data-action="scenario"]').selectOption("partial-error");
  await expect(page.getByText("节点健康摘要暂时无法加载", { exact: false })).toBeVisible();
  await expect(page.locator("tbody tr")).toHaveCount(3);
  await page.getByRole("button", { name: "重新加载" }).click();
  await expect(page.getByText("节点健康摘要暂时无法加载", { exact: false })).toHaveCount(0);
});

test("系统设置、个人资料和筛选状态可恢复", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/settings`);
  await page.getByLabel("同目标并发策略").selectOption("reject");
  await page.getByLabel("日志保留").selectOption("180");
  await page.getByRole("button", { name: "保存设置" }).click();
  await page.reload();
  await expect(page.getByLabel("同目标并发策略")).toHaveValue("reject");
  await expect(page.getByLabel("日志保留")).toHaveValue("180");

  await page.goto(`${baseURL}/#/app/mine/profile`);
  await page.getByLabel("姓名").fill("陈舟新名称");
  await page.getByRole("button", { name: "保存资料" }).click();
  await page.reload();
  await expect(page.getByLabel("姓名")).toHaveValue("陈舟新名称");

  await page.goto(`${baseURL}/#/web/deployments`);
  await page.getByLabel("搜索部署").fill("Atlas");
  await page.getByRole("button", { name: "成功" }).click();
  await page.reload();
  await expect(page.getByLabel("搜索部署")).toHaveValue("Atlas");
  await expect(page.getByRole("button", { name: "成功" })).toHaveAttribute("aria-pressed", "true");
});

test("Agent 节点复检结果在刷新后保持", async ({ page }) => {
  for (const [scenario, result] of [["running", "检查通过"], ["failed", "检查失败"]]) {
    await setScenario(page, scenario, "/web/nodes/node-sh-01");
    await page.getByRole("button", { name: "执行检查" }).click();
    await expect(page.getByText(result, { exact: true }).first()).toBeVisible();
    await page.reload();
    await expect(page.getByText(result, { exact: true }).first()).toBeVisible();
  }
});

test("App 用户操作和通知偏好可恢复", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${baseURL}/#/app/mine/users/new`);
  await page.getByLabel("姓名").fill("移动验收用户");
  await page.getByLabel("登录邮箱").fill("mobile-audit@example.com");
  await page.getByLabel("初始密码").fill("initial-pass-123");
  await page.getByRole("button", { name: "创建用户" }).click();
  await page.getByRole("link", { name: /移动验收用户/ }).click();
  await page.getByRole("button", { name: "停用用户" }).click();
  await page.getByRole("button", { name: "确认停用用户" }).click();
  await expect(page.locator(".status", { hasText: "已停用" })).toBeVisible();
  await page.reload();
  await expect(page.locator(".status", { hasText: "已停用" })).toBeVisible();
  await page.getByRole("button", { name: "启用用户" }).click();
  await expect(page.locator(".status", { hasText: "在线" })).toBeVisible();
  await page.reload();
  await expect(page.locator(".status", { hasText: "在线" })).toBeVisible();

  await page.goto(`${baseURL}/#/app/mine/preferences`);
  await page.getByLabel("部署失败").uncheck();
  await page.reload();
  await expect(page.getByLabel("部署失败")).not.toBeChecked();
});

test("异常场景具有独立反馈和恢复动作", async ({ page }) => {
  await setScenario(page, "loading", "/web/deployments");
  await expect(page.getByLabel("正在加载")).toBeVisible();

  await setScenario(page, "partial-error", "/web/deployments");
  await expect(page.getByText("局部失败不会清除已加载内容")).toBeVisible();
  await expect(page.locator("tbody tr")).toHaveCount(3);
  await page.getByRole("button", { name: "重新加载" }).click();
  await expect(page.getByText("局部失败不会清除已加载内容")).toHaveCount(0);

  await setScenario(page, "full-error", "/web/overview");
  await expect(page.getByRole("heading", { name: "服务暂时不可用" })).toBeVisible();
  await page.getByRole("button", { name: "重新连接" }).click();
  await expect(page.locator(".web-shell")).toBeVisible();

  await setScenario(page, "session-expired", "/app/deployments");
  await expect(page.getByText("会话已失效")).toBeVisible();
  await setScenario(page, "unauthorized", "/app/mine/users");
  await expect(page.getByRole("heading", { name: "没有系统管理权限" })).toBeVisible();
  await setScenario(page, "no-results", "/app/deployments");
  await expect(page.getByRole("heading", { name: "没有匹配结果" })).toBeVisible();
  await setScenario(page, "empty", "/app/overview");
  await expect(page.getByRole("heading", { name: "还没有节点" })).toBeVisible();

  await setScenario(page, "disconnected", "/app/deployments/dep-1042");
  await expect(page.getByText("连接已断开").first()).toBeVisible();
  await expect(page.locator(".mobile-log-line")).not.toHaveCount(0);
  await setScenario(page, "long-log", "/app/deployments/dep-1042");
  expect(await page.locator(".mobile-log-line").count()).toBeGreaterThan(30);
  await expect(page.getByText("DEPLOY_TOKEN=••••••••", { exact: false }).first()).toBeVisible();
});

test("未保存导航保护形成闭环", async ({ page }) => {
  await page.goto(`${baseURL}/#/web/apps/new`);
  await page.getByLabel("应用名称").fill("guard-app");
  await page.getByRole("link", { name: "应用" }).click();
  await expect(page.getByRole("dialog", { name: "放弃未保存的修改？" })).toBeVisible();
  await page.getByRole("button", { name: "继续编辑" }).click();
  await expect(page.getByRole("link", { name: "应用" })).toBeFocused();
});

test("Agent 操作失败保留原状态并允许重试", async ({ page }) => {
  await setScenario(page, "operation-failed", "/web/agents/agent-sh-01");
  await page.getByRole("button", { name: "撤销 Agent" }).click();
  await page.getByRole("button", { name: "确认撤销 Agent" }).click();
  await expect(page.getByRole("alert").getByText("操作没有完成，请重试。")).toBeVisible();
  await expect(page.locator(".status", { hasText: "在线" })).toBeVisible();
  await page.getByRole("button", { name: "确认撤销 Agent" }).click();
  await expect(page.getByRole("button", { name: "处理中…" })).toBeDisabled();
});

test("执行中断保持独立语义并可创建重试任务", async ({ page }) => {
  await setScenario(page, "interrupted", "/web/deployments/dep-1042");
  await expect(page.locator(".status", { hasText: "执行中断" })).toBeVisible();
  await expect(page.getByText(/执行状态未知/)).toBeVisible();
  await page.getByRole("button", { name: "重试部署" }).click();
  await expect(page).toHaveURL(/dep-retry-/);
  await expect(page.getByText("重试来源")).toBeVisible();
});

test("密集列表支持增量加载、筛选摘要和清空", async ({ page }) => {
  await setScenario(page, "dense", "/web/deployments");
  await expect(page.getByText("显示 8 / 24 条部署")).toBeVisible();
  await page.getByRole("button", { name: "加载更多" }).click();
  await expect(page.getByText("显示 14 / 24 条部署")).toBeVisible();
  await page.getByLabel("搜索部署").fill("not-found");
  await expect(page.getByRole("button", { name: "清空筛选" })).toBeVisible();
  await page.getByRole("button", { name: "清空筛选" }).click();
  await expect(page.getByLabel("搜索部署")).toHaveValue("");
});

test("日志工具失败就地反馈且 App 返回固定父页", async ({ page }) => {
  await setScenario(page, "tool-failed", "/app/deployments/dep-1042");
  await page.getByRole("button", { name: "复制日志" }).click();
  await expect(page.getByRole("alert").getByText(/无法复制日志/)).toBeVisible();
  await page.getByRole("button", { name: "返回" }).click();
  await expect(page).toHaveURL(/#\/app\/deployments$/);
});

test("管理员和普通用户的导航与深链权限一致", async ({ page }) => {
  await page.goto(`${baseURL}/#/entry`);
  await page.locator('[data-action="role"]').selectOption("user");
  await page.setViewportSize({ width: 390, height: 844 });
  for (const route of ["overview", "deployments", "apps", "nodes", "mine"]) {
    await page.goto(`${baseURL}/#/app/${route}`);
    await expect(page.locator(".mobile-nav a")).toHaveCount(4);
    await expect(page.locator('.mobile-nav a[aria-current="page"]')).toHaveCount(1);
    await expect(page.getByText("没有系统管理权限")).toHaveCount(0);
  }
  await expect(page.getByText("用户管理")).toHaveCount(0);
  await page.goto(`${baseURL}/#/app/mine/users`);
  await expect(page.getByRole("heading", { name: "没有系统管理权限" })).toBeVisible();
  await page.goto(`${baseURL}/#/web/settings`);
  await expect(page.getByRole("heading", { name: "没有系统管理权限" })).toBeVisible();
});

test("四个目标视口均无页面级横向溢出", async ({ page }) => {
  for (const [width, height, prefix, routes] of [[1440, 900, "web", webRoutes], [1024, 768, "web", webRoutes], [390, 844, "app", appRoutes], [360, 800, "app", appRoutes]]) {
    await page.setViewportSize({ width, height });
    for (const route of routes) {
      await page.goto(`${baseURL}/#/${prefix}/${route}`);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBeTruthy();
      if (prefix === "app") expect(await page.locator(".mobile-content").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
    }
  }
});

test("App 触摸目标、大字体和键盘焦点符合移动规范", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  for (const route of appRoutes) {
    await page.goto(`${baseURL}/#/app/${route}`);
    const undersized = await page.locator(".mobile-shell").evaluate((root) => [...root.querySelectorAll("a[href], button, input, select")]
      .filter((element) => !element.closest(".toggle-row"))
      .filter((element) => {
        const box = element.getBoundingClientRect();
        return box.width > 0 && box.height > 0 && (box.width < 44 || box.height < 44);
      }).map((element) => element.getAttribute("aria-label") || element.textContent.trim()));
    expect(undersized).toEqual([]);

    await page.locator(".mobile-shell").evaluate((root) => {
      for (const element of root.querySelectorAll("h1, h2, p, span, strong, small, label, button, a, input")) {
        element.style.fontSize = `${Number.parseFloat(getComputedStyle(element).fontSize) * 1.3}px`;
        element.style.letterSpacing = "0px";
      }
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBeTruthy();
  }

  await page.goto(`${baseURL}/#/app/deployments`);
  const deploymentSearch = page.getByLabel("搜索部署");
  await expect(deploymentSearch).toBeVisible();
  await expect(page.locator("main")).toBeFocused();
  await deploymentSearch.focus();
  await expect(deploymentSearch).toBeFocused();
  await expect(page.locator(".mobile-search")).toHaveCSS("outline-style", "solid");
});

test("200% 缩放等效视口保持 Web 回流", async ({ page }) => {
  await page.setViewportSize({ width: 720, height: 450 });
  for (const route of webRoutes) {
    await page.goto(`${baseURL}/#/web/${route}`);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBeTruthy();
    expect(await page.locator(".web-main").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
  }
});
