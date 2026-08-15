import { expect, test, type Page, type Route } from "@playwright/test";

const administrator = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const deployment = { id: "deployment-20260815-very-long", application_id: "app-1", target_id: "target-production-very-long", target_runs: [], requested_by: "admin-1", status: "running", phase: "release", execution_mode: "script", stage_tasks: [], snapshot_hash: "snapshot-1", protocol_complete: false, queued_at: "2026-08-15T04:00:00Z", started_at: "2026-08-15T04:00:01Z", created_at: "2026-08-15T04:00:00Z", updated_at: "2026-08-15T04:00:01Z", version: 1 };
const node = { id: "node-1", name: "生产节点 01", host: null, port: null, username: null, ssh_credential_id: null, work_root: "/var/lib/deploy-go-agent/apps", secrets_root: "/var/lib/deploy-go-agent/secrets", status: "online", trusted_host_fingerprint: null, checked_at: null, version: 1, created_at: "2026-08-15T04:00:00Z", updated_at: "2026-08-15T04:00:00Z" };
const agent = { id: "agent-1", node_id: "node-1", name: "生产节点 01", environment: "test", status: "online", registered_at: "2026-08-15T04:00:00Z", last_seen_at: "2026-08-15T04:07:00Z", agent_version: "11.0.0", hostname: "qfy-test-1", architecture: "x86_64", revoked_at: null, created_at: "2026-08-15T04:00:00Z" };
const application = { id: "app-1", name: "卡券系统正式环境", slug: "voucher-hub-production", description: "用于验证控制面数据密集列表在窄屏下的优先级。", environment: "prod", status: "active", version: 1, created_at: "2026-08-15T04:00:00Z", updated_at: "2026-08-15T04:00:00Z" };

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function authenticate(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, administrator));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-layout" }));
  await page.route("**/api/v1/deployments?**", (route) => json(route, { items: [deployment], next_cursor: null }));
  await page.route("**/api/v1/nodes?**", (route) => json(route, { items: [node], next_cursor: null }));
  await page.route("**/api/v1/nodes/node-1", (route) => json(route, node));
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [agent], next_cursor: null }));
  await page.route("**/api/v1/nodes/node-1/terminal-capability", (route) => json(route, {
    node_id: "node-1",
    available: true,
    unavailable_code: null,
    agent_id: "agent-1",
    agent_online: true,
    identity_valid: true,
    protocol_version: 11,
    pty_terminal: true,
  }));
  await page.route("**/api/v1/applications?**", (route) => json(route, { items: [application], next_cursor: null }));
}

async function expectNoViewportOverflow(page: Page) {
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth)).toBe(false);
}

test("高频列表在桌面完整展示，窄屏保留关键字段且无页面级溢出", async ({ page }, testInfo) => {
  await authenticate(page);

  await page.goto("/deployments");
  await expect(page.getByRole("heading", { name: "部署记录" })).toBeVisible();
  await expect(page.locator(".deployment-table th.table-column--secondary").first()).toBeVisible();
  await expectNoViewportOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("deployments-desktop.png"), fullPage: true });

  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator(".deployment-table th.table-column--secondary").first()).toBeHidden();
  await expect(page.getByRole("link", { name: "查看" })).toBeVisible();
  await expectNoViewportOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("deployments-mobile.png"), fullPage: true });

  await page.goto("/nodes");
  await expect(page.locator(".workspace-heading h2", { hasText: "节点" })).toBeVisible();
  await expect(page.locator(".data-table th.table-column--secondary").first()).toBeHidden();
  await expect(page.getByRole("link", { name: "管理" })).toBeVisible();
  await expectNoViewportOverflow(page);

  await page.goto("/apps");
  await expect(page.locator(".workspace-heading h2", { hasText: "应用" })).toBeVisible();
  await expect(page.locator(".data-table th.table-column--secondary").first()).toBeHidden();
  await expect(page.getByRole("link", { name: "配置" })).toBeVisible();
  await expectNoViewportOverflow(page);
});

test("节点详情使用完整工作区宽度且在窄屏保持可用", async ({ page }, testInfo) => {
  await authenticate(page);
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("/nodes/node-1");
  await expect(page.getByRole("heading", { name: "生产节点 01" })).toBeVisible();

  const widths = await page.locator(".detail-page").evaluate((detail) => {
    const content = detail.parentElement;
    if (!content) throw new Error("缺少页面内容容器");
    const style = getComputedStyle(content);
    return {
      detail: detail.getBoundingClientRect().width,
      content: content.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
    };
  });
  expect(widths.detail).toBeGreaterThanOrEqual(widths.content - 1);
  const lastDefinitionItemFillsRow = await page.locator(".detail-page [role='tabpanel'][aria-label='概览'] > .definition-grid > div:nth-child(3)").evaluate((item) => {
    const grid = item.parentElement;
    if (!grid) throw new Error("缺少定义网格容器");
    return item.getBoundingClientRect().width >= grid.getBoundingClientRect().width - 3;
  });
  expect(lastDefinitionItemFillsRow).toBe(true);
  await expectNoViewportOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("node-detail-desktop.png"), fullPage: true });

  await page.setViewportSize({ width: 390, height: 844 });
  await expectNoViewportOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("node-detail-mobile.png"), fullPage: true });
});
