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

test("管理员配置镜像直连目标并提交特权 image_spec", async ({ page }) => {
  await auth(page);
  const node = {
    id: "node-1",
    name: "Node",
    host: "node.fixture.invalid",
    port: 22,
    username: "deploy",
    ssh_credential_id: "cred-1",
    work_root: "/srv/apps",
    secrets_root: "/srv/secrets",
    status: "online",
    trusted_host_fingerprint: "SHA256:host",
    checked_at: "2026-08-01T00:00:00Z",
    version: 1,
    created_at: "2026-08-01T00:00:00Z",
    updated_at: "2026-08-01T00:00:00Z",
  };
  const composeEnvFile = {
    id: "env-compose",
    file_name: "compose.env",
    module: "compose",
    format: "dotenv-v1",
    current_version: 1,
    current_digest: "a".repeat(64),
    declared_at: "2026-08-01T00:00:00Z",
    updated_at: "2026-08-01T00:00:00Z",
    target_count: 0,
    pending_count: 0,
    syncing_count: 0,
    succeeded_count: 0,
    failed_count: 0,
    syncs: [],
  };
  const redisEnvFile = {
    id: "env-redis",
    file_name: "redis.env",
    module: "redis",
    format: "dotenv-v1",
    current_version: 1,
    current_digest: "a".repeat(64),
    declared_at: "2026-08-01T00:00:00Z",
    updated_at: "2026-08-01T00:00:00Z",
    target_count: 0,
    pending_count: 0,
    syncing_count: 0,
    succeeded_count: 0,
    failed_count: 0,
    syncs: [],
  };
  let targets: unknown[] = [];
  let targetBody: Record<string, unknown> | undefined;
  await page.route("**/api/v1/applications/app-1", (route) => json(route, application));
  await page.route("**/api/v1/applications/app-1/targets*", async (route) => {
    if (route.request().method() === "POST") {
      expect(route.request().headers()["x-csrf-token"]).toBe("csrf-app-e2e");
      targetBody = await route.request().postDataJSON() as Record<string, unknown>;
      const created = { id: "target-image-1", application_id: "app-1", ...targetBody, status: "active", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
      targets = [created];
      await json(route, created, 201);
      return;
    }
    await json(route, { items: targets, next_cursor: null });
  });
  await page.route("**/api/v1/applications/app-1/env-files*", (route) => json(route, { items: [composeEnvFile, redisEnvFile], next_cursor: null }));
  await page.route("**/api/v1/applications/app-1/source", (route) => json(route, { code: "not_found", message: "应用来源不存在", request_id: "req-source-e2e" }, 404));
  await page.route("**/api/v1/git-credentials?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/agents?**", (route) => json(route, { items: [], next_cursor: null }));
  await page.route("**/api/v1/nodes?**", (route) => json(route, { items: [node], next_cursor: null }));

  await page.goto("/apps/app-1");
  await page.getByRole("button", { name: "添加目标" }).click();
  await page.getByLabel("节点").click();
  await page.getByRole("option", { name: "Node · node.fixture.invalid" }).click();
  await page.getByLabel("执行模式").click();
  await page.getByRole("option", { name: "镜像直连模式（模板 + 官方镜像）" }).click();
  await expect(page.getByLabel("镜像引用")).toHaveValue("docker.io/library/redis:7-alpine");
  await expect(page.getByRole("spinbutton", { name: "宿主端口" })).toHaveValue("6379");
  await page.getByRole("checkbox", { name: /compose\.env/ }).check();
  await page.getByRole("checkbox", { name: /redis\.env/ }).check();
  await page.getByRole("checkbox", { name: /我确认该镜像、模板与宿主端口/ }).check();
  await page.getByRole("button", { name: "保存目标" }).click();

  await expect(page.getByText("docker.io/library/redis:7-alpine")).toBeVisible();
  await expect(page.getByText("镜像直连")).toBeVisible();
  expect(targetBody).toMatchObject({
    node_id: "node-1",
    execution_mode: "image",
    privileged_release: true,
    privileged_release_confirmed: true,
    image_spec: { template: "redis", image: "docker.io/library/redis:7-alpine", host_port: 6379, env_files: ["compose.env", "redis.env"] },
  });
  expect(targetBody!.secret_file_references).toEqual([]);
});
