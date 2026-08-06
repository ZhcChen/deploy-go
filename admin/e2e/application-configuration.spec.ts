import { expect, test, type Page, type Route } from "@playwright/test";

const admin = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const application = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
async function json(route: Route, body: unknown, status = 200) { await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) }); }
async function auth(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-app-e2e" }));
}

test("管理员创建应用并进入部署目标配置", async ({ page }) => {
  await auth(page);
  let items: typeof application[] = [];
  await page.route("**/api/v1/applications*", async (route) => {
    if (route.request().method() === "POST") {
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-app-e2e");
      expect(await route.request().postDataJSON()).toMatchObject({ name: "Voucher Hub", slug: "voucher-hub" });
      items = [application];
      await json(route, application, 201);
      return;
    }
    await json(route, { items, next_cursor: null });
  });
  await page.route("**/api/v1/applications/app-1", (route) => json(route, application));
  await page.route("**/api/v1/applications/app-1/targets?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/applications/app-1/source", (route) => json(route, { code: "not_found", message: "应用来源不存在", request_id: "req-source-e2e" }, 404));
  await page.route("**/api/v1/git-credentials?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/nodes?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.goto("/apps");
  await page.getByRole("button", { name: "创建应用" }).click();
  await page.getByLabel("应用名称").fill("Voucher Hub");
  await page.getByLabel("Slug").fill("voucher-hub");
  await page.getByLabel("说明").fill("代金券服务");
  await page.getByRole("button", { name: "保存应用" }).click();
  await page.getByRole("link", { name: "配置" }).click();
  await expect(page).toHaveURL(/\/apps\/app-1$/);
  await expect(page.getByRole("button", { name: "添加目标" })).toBeVisible();
});

test("管理员为普通用户分配应用", async ({ page }) => {
  await auth(page);
  let granted = false;
  await page.route("**/api/v1/users?**", (route) => json(route, { items: [{ id: "user-1", username: "operator", display_name: "部署用户", identity: "user", status: "active", version: 1 }], next_cursor: null }));
  await page.route("**/api/v1/applications?**", (route) => json(route, { items: [application], next_cursor: null }));
  await page.route("**/api/v1/users/user-1/applications?**", (route) => json(route, { items: granted ? [{ application_id: "app-1", granted_at: "2026-08-01T00:00:00Z" }] : [], next_cursor: null }));
  await page.route("**/api/v1/users/user-1/applications/app-1", async (route) => { granted = true; await route.fulfill({ status: 204 }); });
  await page.goto("/settings/application-access");
  await page.getByRole("button", { name: /部署用户/ }).click();
  await page.getByRole("button", { name: /Voucher Hub/ }).click();
  await expect(page.getByRole("button", { name: /Voucher Hub/ })).toHaveAttribute("aria-pressed", "true");
});
