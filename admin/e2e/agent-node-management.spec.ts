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
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [agent], next_cursor: null }));
}

test("管理员通过 Agent 执行节点能力检查", async ({ page }) => {
  await authenticatedApi(page);
  await page.route("**/api/v1/nodes/node-1/checks", async (route) => {
    expect(route.request().headers()["x-csrf-token"]).toBe("test-csrf");
    await json(route, { id: "check-1", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 21474836480, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, 201);
  });
  await page.goto("/nodes/node-1");
  await expect(page.getByRole("link", { name: "查看 Agent" })).toHaveAttribute("href", "/agents/agent-1");
  await page.getByRole("button", { name: "执行检查" }).click();
  await expect(page.getByText("20.0 GiB")).toBeVisible();
});
