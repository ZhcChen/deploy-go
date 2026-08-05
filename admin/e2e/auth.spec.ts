import { expect, test, type Page, type Route } from "@playwright/test";

const admin = {
  id: "admin-1",
  username: "admin",
  display_name: "陈舟",
  identity: "administrator",
};

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function anonymousApi(page: Page) {
  await page.route("**/api/v1/setup", (route) =>
    route.request().method() === "GET"
      ? json(route, { setup_required: false })
      : route.fallback(),
  );
  await page.route("**/api/v1/auth/me", (route) =>
    json(route, { code: "not_authenticated", message: "未登录", request_id: "req-e2e" }, 401),
  );
  await page.route("**/api/v1/deployments?**", (route) =>
    json(route, { items: [], next_cursor: null }),
  );
}

test("登录后返回原部署页面且 Origin 使用当前站点", async ({ page }) => {
  await anonymousApi(page);
  await page.route("**/api/v1/auth/login", async (route) => {
    expect(route.request().headers().origin).toBe("http://127.0.0.1:5174");
    await json(route, { csrf_token: "csrf-login", user: admin });
  });
  await page.goto("/deployments");
  await page.getByLabel("账号或邮箱").fill("admin");
  await page.getByLabel("密码").fill("password123");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page).toHaveURL(/\/deployments$/);
  await expect(page.getByRole("heading", { level: 1, name: "部署" })).toBeVisible();
});

test("登录关键流程可仅用键盘完成", async ({ page }) => {
  await anonymousApi(page);
  await page.route("**/api/v1/auth/login", (route) => json(route, { csrf_token: "csrf-keyboard", user: admin }));
  await page.goto("/deployments");
  await page.getByLabel("账号或邮箱").focus();
  await page.keyboard.type("admin");
  await page.keyboard.press("Tab");
  await page.keyboard.type("password123");
  await page.keyboard.press("Enter");

  await expect(page).toHaveURL(/\/deployments$/);
});

test("管理员可退出并清除本地会话状态", async ({ page }) => {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-refresh" }));
  await page.route("**/api/v1/auth/logout", async (route) => {
    expect(route.request().headers()["x-csrf-token"]).toBe("csrf-refresh");
    await route.fulfill({ status: 204 });
  });
  await page.goto("/overview");
  await page.getByRole("button", { name: "退出登录" }).click();
  await expect(page).toHaveURL(/\/login$/);
});

test("普通用户无设置导航且直接访问显示 403", async ({ page }) => {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, { ...admin, identity: "user", display_name: "林臻" }));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-user" }));
  await page.goto("/settings/users");
  await expect(page.getByRole("heading", { name: "没有访问权限" })).toBeVisible();
  await expect(page.getByRole("link", { name: "设置" })).toHaveCount(0);
});

test("首次初始化提交后进入登录且页面不保留输入", async ({ page }) => {
  await page.route("**/api/v1/setup", async (route) => {
    if (route.request().method() === "GET") {
      await json(route, { setup_required: true });
      return;
    }
    expect(route.request().headers()["x-setup-token"]).toBeUndefined();
    await json(route, admin, 201);
  });
  await page.goto("/setup");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByLabel("初始密码").fill("password123");
  await page.getByRole("button", { name: "完成初始化" }).click();
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.locator("body")).not.toContainText("password123");
});
