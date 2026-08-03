import { expect, test, type Page, type Route } from "@playwright/test";

const administrator = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const agent = { id: "agent-1", node_id: "node-1", name: "生产节点 01", status: "offline", registered_at: null, last_seen_at: null, agent_version: null, hostname: null, architecture: null, revoked_at: null, created_at: "2026-08-03T00:00:00Z" };
const installCommand = "printf '%s\\n' 'dga_enroll_fixture' | sudo bash";

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function authenticate(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false, setup_enabled: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, administrator));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-agent-e2e" }));
}

test("管理员创建 Agent 并获得一次性安装命令", async ({ page }) => {
  await authenticate(page);
  let created = false;
  await page.route("**/api/v1/agents**", async (route) => {
    if (route.request().method() === "POST" && new URL(route.request().url()).pathname === "/api/v1/agents") {
      created = true;
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-agent-e2e");
      await json(route, { agent, enrollment_token: "dga_enroll_fixture", enrollment_expires_at: "2026-08-03T08:00:00Z", install_command: installCommand }, 201);
      return;
    }
    await json(route, { items: created ? [agent] : [], next_cursor: null });
  });

  await page.goto("/agents");
  await page.getByRole("button", { name: "创建 Agent" }).click();
  await page.getByLabel("Agent 名称").fill("生产节点 01");
  await page.getByRole("button", { name: "创建并生成命令" }).click();
  await expect(page.getByText(installCommand)).toBeVisible();
  await expect(page.getByText(/当前离线/)).toBeVisible();
  await expect(page.locator("body")).not.toContainText("access_token");
  await expect(page.locator("body")).not.toContainText("refresh_token");
});

test("窄屏 Agent 详情命令不造成页面级溢出", async ({ page }) => {
  await authenticate(page);
  await page.setViewportSize({ width: 720, height: 900 });
  await page.route("**/api/v1/agents/agent-1", (route) => json(route, agent));
  await page.goto("/agents/agent-1");
  await expect(page.getByRole("heading", { name: "生产节点 01" })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
});
