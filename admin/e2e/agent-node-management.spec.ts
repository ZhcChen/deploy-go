import { expect, test, type Page, type Route } from "@playwright/test";

const admin = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const node = { id: "node-1", name: "生产节点", host: null, port: null, username: null, ssh_credential_id: null, work_root: "/var/lib/deploy-go-agent/apps", secrets_root: "/var/lib/deploy-go-agent/secrets", status: "online", trusted_host_fingerprint: null, checked_at: null, version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
const agent = { id: "agent-1", node_id: "node-1", name: "生产 Agent", status: "online", agent_version: "0.1.0", hostname: "prod-01", architecture: "x86_64", created_at: "2026-08-01T00:00:00Z" };

async function json(route: Route, body: unknown, status = 200) { await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) }); }
async function authenticatedApi(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "test-csrf" }));
  await page.route("**/api/v1/nodes/node-1", (route) => json(route, node));
  await page.route("**/api/v1/nodes/node-1/telemetry", (route) => json(route, { node_id:"node-1",connectivity:"online",capability:"supported",freshness:"empty",captured_at:null,received_at:null,latest:null,history:[] }));
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [agent], next_cursor: null }));
  await page.route("**/api/v1/nodes/node-1/terminal-capability", (route) => json(route, {
    node_id: "node-1",
    available: false,
    unavailable_code: "terminal_executor_unavailable",
    agent_id: "agent-1",
    agent_online: true,
    identity_valid: true,
    protocol_version: 11,
    pty_terminal: true,
  }));
}

test("管理员通过节点协同程序执行能力检查", async ({ page }) => {
  await authenticatedApi(page);
  await page.route("**/api/v1/nodes/node-1/checks", async (route) => {
    expect(route.request().headers()["x-csrf-token"]).toBe("test-csrf");
    await json(route, { id: "check-1", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 21474836480, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, 201);
  });
  await page.goto("/nodes/node-1");
  await expect(page.getByRole("heading", { name: "节点协同程序" })).toBeVisible();
  await expect(page.getByText("v0.1.0")).toBeVisible();
  await page.getByRole("button", { name: "执行检查" }).click();
  await expect(page.getByText("20.0 GiB")).toBeVisible();
});

test("节点 SSH 门禁在桌面与窄屏保持清晰且支持深链", async ({ page }) => {
  await authenticatedApi(page);
  await page.goto("/nodes/node-1");
  await page.getByRole("tab", { name: "SSH" }).click();
  await expect(page).toHaveURL(/\/nodes\/node-1\?view=ssh$/);
  await expect(page.getByRole("heading", { name: "节点终端 executor 不可用" })).toBeVisible();

  for (const viewport of [{ width: 1280, height: 800 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(overflow).toBe(false);
  }
});
