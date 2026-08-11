import { expect, test, type Page, type Route } from "@playwright/test";

const admin = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const envFile = { id: "env-1", application_id: "app-1", file_name: "api.env", module: "api", format: "dotenv-v1", current_version: 3, current_digest: "a".repeat(64), declared_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-06T03:00:00Z", version: 4, target_count: 2, pending_count: 1, syncing_count: 0, succeeded_count: 1, failed_count: 0, syncs: [{ target_id: "target-pending", node_id: "node-pending", node_name: "Node Pending", status: "pending", actual_version: null, last_attempt_at: null, synced_at: null, error_code: null, error_message: null }, { target_id: "target-success", node_id: "node-success", node_name: "Node Success", status: "succeeded", actual_version: 3, last_attempt_at: "2026-08-06T03:00:00Z", synced_at: "2026-08-06T03:00:01Z", error_code: null, error_message: null }] };
const plaintext = { id: "env-1", application_id: "app-1", file_name: "api.env", module: "api", format: "dotenv-v1", content: "# API\nPORT=8080\nTOKEN=top-secret\n", digest: "a".repeat(64), env_version: 3, version: 4, updated_at: "2026-08-06T03:00:00Z" };

async function json(route: Route, body: unknown, status = 200) { await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) }); }
async function auth(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-env-e2e" }));
}

test("管理员重新认证、校验并保存已有 Env", async ({ page }) => {
  await auth(page);
  await page.route("**/api/v1/applications/app-1/env-files", (route) => json(route, { items: [envFile] }));
  await page.route("**/api/v1/applications/app-1/env-reveal-grants", async (route) => {
    expect(await route.request().postDataJSON()).toEqual({ password: "password", action: "read_write" });
    await json(route, { action: "read_write", grant_token: "grant-read", expires_at: "2099-08-06T03:05:00Z" });
  });
  await page.route("**/api/v1/application-env-files/env-1", async (route) => {
    expect(route.request().headers()["x-env-reveal-grant"]).toBe("grant-read");
    if (route.request().method() === "PUT") {
      const body = await route.request().postDataJSON();
      expect(body.content).toContain("PORT=9090");
      await json(route, { ...plaintext, ...body, env_version: 4, version: 5 });
      return;
    }
    await json(route, plaintext);
  });
  await page.goto("/apps/app-1/config/env-1");
  await page.getByLabel("管理员密码").fill("password");
  await page.getByRole("button", { name: "验证并读取" }).click();
  await expect(page.getByLabel("PORT 的值")).toHaveValue("8080");
  await page.getByRole("button", { name: "原文模式" }).click();
  await page.getByLabel("api.env 原文").fill("# API\nPORT=9090\nTOKEN=new-secret\nPORT=7070\n");
  await expect(page.getByRole("alert")).toContainText("第 4 行");
  await expect(page.getByRole("button", { name: "保存 Env" })).toBeDisabled();
  await page.getByLabel("api.env 原文").fill("# API\nPORT=9090\nTOKEN=new-secret\n");
  await page.getByRole("button", { name: "保存 Env" }).click();
  const dialog = page.getByRole("dialog", { name: "保存 api.env？" });
  await expect(dialog).toContainText("~ PORT=••••••");
  await expect(dialog).not.toContainText("new-secret");
  await dialog.getByRole("button", { name: "确认保存" }).click();
  await expect(page.getByLabel("api.env 原文")).toHaveValue("# API\nPORT=9090\nTOKEN=new-secret\n");
  const persisted = await page.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } }));
  expect(persisted).not.toContain("new-secret");
});
