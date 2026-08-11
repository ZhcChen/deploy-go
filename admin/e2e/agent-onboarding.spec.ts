import { expect, test, type Page, type Route } from "@playwright/test";

const administrator = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const agent = { id: "agent-1", node_id: "node-1", name: "生产节点 01", environment: "prod", status: "offline", registered_at: null, last_seen_at: null, agent_version: null, hostname: null, architecture: null, revoked_at: null, created_at: "2026-08-03T00:00:00Z" };
const installCommand = "sudo env 'DEPLOY_GO_AGENT_ID=agent-1' 'DEPLOY_GO_AGENT_ENROLLMENT_TOKEN=dga_enroll_fixture' bash";

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function authenticate(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, administrator));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-agent-e2e" }));
}

test("管理员创建节点并获得一次性安装命令", async ({ page }) => {
  await authenticate(page);
  let created = false;
  await page.route("**/api/v1/nodes**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/agents**", async (route) => {
    if (route.request().method() === "POST" && new URL(route.request().url()).pathname === "/api/v1/agents") {
      created = true;
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-agent-e2e");
      await json(route, { agent, enrollment_token: "dga_enroll_fixture", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: installCommand }, 201);
      return;
    }
    await json(route, { items: created ? [agent] : [], next_cursor: null });
  });

  await page.goto("/nodes");
  await page.getByRole("button", { name: "创建节点" }).click();
  await page.getByLabel("节点名称").fill("生产节点 01");
  await page.getByRole("button", { name: "环境", exact: true }).click();
  await page.getByRole("option", { name: "生产环境" }).click();
  await page.getByRole("button", { name: "创建并生成安装命令" }).click();
  await expect(page.getByText(installCommand)).toBeVisible();
  expect(installCommand).toContain("dga_enroll_fixture");
  await expect(page.getByText(/目标 Linux 服务器/)).toBeVisible();
  await expect(page.locator("body")).not.toContainText("access_token");
  await expect(page.locator("body")).not.toContainText("refresh_token");
});

test("窄屏节点详情不造成页面级溢出", async ({ page }) => {
  await authenticate(page);
  await page.setViewportSize({ width: 720, height: 900 });
  await page.route("**/api/v1/nodes/node-1", (route) => json(route, { id: "node-1", name: "生产节点 01", status: "offline", work_root: "/var/lib/deploy-go-agent/apps", secrets_root: "/var/lib/deploy-go-agent/secrets", version: 1 }));
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [agent], next_cursor: null }));
  await page.goto("/nodes/node-1");
  await expect(page.getByRole("heading", { name: "生产节点 01" })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
});
