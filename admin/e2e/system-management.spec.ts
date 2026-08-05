import { expect, test, type Page, type Route } from "@playwright/test";

const administrator = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const ordinaryUser = { id: "user-1", username: "operator", display_name: "部署用户", email: "operator@example.invalid", identity: "user" };

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function authenticate(page: Page, user = administrator) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, user));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-system-e2e" }));
}

test("管理员设置初始密码创建普通用户", async ({ page }) => {
  await authenticate(page);
  let requestBody: unknown;
  await page.route("**/api/v1/users*", async (route) => {
    if (route.request().method() === "POST") {
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-system-e2e");
      requestBody = await route.request().postDataJSON();
      await json(route, { ...ordinaryUser, status: "active", version: 1 }, 201);
      return;
    }
    await json(route, { items: [], next_cursor: null });
  });

  await page.goto("/settings/users");
  await page.getByRole("button", { name: "创建用户" }).click();
  await page.getByLabel("用户名").fill("operator");
  await page.getByLabel("显示名称").fill("部署用户");
  await page.getByLabel("邮箱").fill("operator@example.invalid");
  await page.getByLabel("初始密码").fill("initial-pass-123");
  await page.locator("form").getByRole("button", { name: "创建用户" }).click();

  await expect.poll(() => requestBody).toEqual({
    username: "operator",
    display_name: "部署用户",
    email: "operator@example.invalid",
    password: "initial-pass-123",
  });
  await expect(page.locator("body")).not.toContainText("initial-pass-123");
  await expect(page.getByText(/邀请用户|角色管理/)).toHaveCount(0);
  await expect(page).not.toHaveURL(/initial-pass-123/);
});

test("管理员修改运行设置时提交当前版本", async ({ page }) => {
  await authenticate(page);
  const initial = { max_concurrent_deployments: 2, max_log_bytes: 52428800, log_retention_days: 30, version: 1 };
  let requestBody: unknown;
  await page.route("**/api/v1/settings", async (route) => {
    if (route.request().method() === "PATCH") {
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-system-e2e");
      requestBody = await route.request().postDataJSON();
      await json(route, { ...initial, max_concurrent_deployments: 4, version: 2 });
      return;
    }
    await json(route, initial);
  });

  await page.goto("/settings");
  await page.getByLabel(/^最大并发部署数/).fill("4");
  await page.getByRole("button", { name: "保存设置" }).click();
  await expect.poll(() => requestBody).toEqual({ ...initial, max_concurrent_deployments: 4 });
});

test("普通用户可保存个人资料但不能访问系统设置", async ({ page }) => {
  await authenticate(page, ordinaryUser);
  let displayName = ordinaryUser.display_name;
  await page.route("**/api/v1/auth/profile", async (route) => {
    if (route.request().method() === "PATCH") {
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-system-e2e");
      displayName = (await route.request().postDataJSON() as { display_name: string }).display_name;
    }
    await json(route, { ...ordinaryUser, display_name: displayName });
  });
  await page.route("**/api/v1/auth/preferences", (route) => json(route, {
    notify_deployment_failed: true,
    notify_deployment_completed: false,
    notify_node_unhealthy: true,
    time_format: "24h",
    follow_logs: true,
    version: 1,
  }));

  await page.goto("/profile");
  await page.getByLabel("显示名称").fill("值班用户");
  await page.getByRole("button", { name: "保存资料" }).click();
  await expect(page.getByText("值班用户", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("权限说明")).toHaveCount(0);

  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: "没有访问权限" })).toBeVisible();
  await expect(page.getByRole("link", { name: "设置" })).toHaveCount(0);
});
