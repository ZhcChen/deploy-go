import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

const admin = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const application = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z", last_deployed_at: "2026-08-02T00:00:00Z" };
const target = { id: "target-1", application_id: "app-1", node_id: "node-1", environment: "production", execution_mode: "script", script_path: "scripts/deploy.sh", parameter_schema: { type: "object", required: ["release-version"], properties: { "release-version": { type: "string", title: "发布版本" } } }, secret_file_references: [], verification_config: {}, timeout_seconds: 600, status: "active", snapshot_hash: "target-snapshot", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const targetTwo = { ...target, id: "target-2", node_id: "node-2", script_path: "scripts/deploy-secondary.sh" };
const targetRuns = [
  { id: "run-1", target_id: "target-1", node_id: "node-1", agent_id: "agent-1", status: "running", phase: "release", env_gate_status: "ready", result_summary: null, error_code: null, source_run_id: null, started_at: "2026-08-02T00:00:01Z", finished_at: null, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:01Z" },
  { id: "run-2", target_id: "target-2", node_id: "node-2", agent_id: "agent-2", status: "pending", phase: "pending", env_gate_status: "pending", result_summary: null, error_code: null, source_run_id: null, started_at: null, finished_at: null, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:01Z" },
];
const deployment = { id: "deployment-1", application_id: "app-1", target_id: "target-1", target_runs: targetRuns, requested_by: "admin-1", status: "running", phase: "targets_running", execution_mode: "script", stage_tasks: [], snapshot_hash: "preview-snapshot", protocol_complete: false, queued_at: "2026-08-02T00:00:00Z", started_at: "2026-08-02T00:00:01Z", created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:01Z", version: 1 };

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

async function authenticate(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "test-csrf" }));
}

test("preview 后确认部署并安全展示实时日志", async ({ page }) => {
  await authenticate(page);
  let confirmRequest: { headers: Record<string, string>; body: unknown } | undefined;
  let cancelCalls = 0;
  await page.route("**/api/v1/applications?**", (route) => json(route, { items: [application], next_cursor: null }));
  await page.route("**/api/v1/applications/app-1/targets?**", (route) => json(route, { items: [target, targetTwo], next_cursor: null }));
  await page.route("**/api/v1/applications/app-1/deployment-preview", (route) => json(route, { application_id: "app-1", application_name: "Voucher Hub", execution_mode: "script", parameters: { "release-version": "v1.2.3" }, snapshot_hash: "preview-snapshot", targets: [
    { target_id: "target-1", node_id: "node-1", node_name: "prod-01", agent_id: "agent-1", agent_online: true, env_gate_status: "ready", script_path: "scripts/deploy.sh" },
    { target_id: "target-2", node_id: "node-2", node_name: "prod-02", agent_id: "agent-2", agent_online: false, env_gate_status: "pending", script_path: "scripts/deploy-secondary.sh" },
  ] }));
  await page.route("**/api/v1/applications/app-1/deployments", async (route) => {
    confirmRequest = { headers: route.request().headers(), body: await route.request().postDataJSON() };
    await json(route, deployment, 201);
  });
  await page.route("**/api/v1/deployments/deployment-1", (route) => json(route, deployment));
  await page.route("**/api/v1/deployments/deployment-1/logs", (route) => route.fulfill({ contentType: "text/event-stream", body: "id: 1\nevent: log\ndata: {\"sequence\":1,\"stream\":\"stdout\",\"content\":\"<img src=x onerror=alert(1)>\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:02Z\"}\n\n" }));
  await page.route("**/api/v1/deployments/deployment-1/cancel", async (route) => { cancelCalls += 1; await json(route, { ...deployment, status: "canceling", phase: "canceling", version: 2 }); });

  await page.goto("/deployments/new?application=app-1");
  await page.getByLabel("发布版本").fill("v1.2.3");
  await page.getByRole("button", { name: "生成部署预览" }).click();
  await expect(page.getByText("preview-snapshot")).toBeVisible();
  await expect(page.getByText("prod-02")).toBeVisible();
  await expect(page.getByText("离线，部署将等待节点恢复")).toBeVisible();
  await expect(page.getByRole("listbox", { name: "选择应用" })).toBeVisible();
  const applicationListHeight = await page.locator(".deployment-application-list").evaluate((list) => list.clientHeight);
  expect(applicationListHeight).toBeGreaterThan(300);
  await page.getByRole("button", { name: /确认并发起部署/ }).click();
  await expect(page).toHaveURL(/\/deployments\/deployment-1$/);
  await page.getByRole("tab", { name: "日志" }).click();
  await expect(page).toHaveURL(/view=logs/);
  await expect(page.getByText("<img src=x onerror=alert(1)>")).toBeVisible();
  await expect(page.locator(".log-viewport img")).toHaveCount(0);
  expect(confirmRequest?.headers["x-csrf-token"]).toBe("test-csrf");
  expect(confirmRequest?.headers["idempotency-key"]).toMatch(/^deploy-[0-9a-f-]{36}$/);
  expect(confirmRequest?.body).toEqual({ parameters: { "release-version": "v1.2.3" }, snapshot_hash: "preview-snapshot", release_strategy: "automatic" });
  const cancelTrigger = page.getByRole("button", { name: "取消部署" });
  await cancelTrigger.click();
  const dialog = page.getByRole("dialog", { name: "取消部署" });
  await expect(dialog).toBeVisible();
  await expect(page.getByRole("button", { name: "返回" })).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByRole("button", { name: "确认取消" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "返回" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(cancelTrigger).toBeFocused();
  await cancelTrigger.press("Enter");
  await page.getByRole("button", { name: "确认取消" }).press("Enter");
  await expect.poll(() => cancelCalls).toBe(1);
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("link", { name: "返回部署" })).toBeFocused();
});

test("部署页应用列表占满剩余高度并在自身区域内滚动", async ({ page }) => {
  await authenticate(page);
  const applications = Array.from({ length: 40 }, (_, index) => ({
    ...application,
    id: `app-${index + 1}`,
    name: `应用 ${index + 1}`,
    slug: `application-${index + 1}`,
    last_deployed_at: new Date(Date.UTC(2026, 7, 28, 0, index + 1)).toISOString(),
  }));
  await page.route("**/api/v1/applications?**", (route) => json(route, { items: applications, next_cursor: null }));
  await page.route("**/api/v1/applications/*/targets?**", (route) => json(route, { items: [target], next_cursor: null }));

  await page.goto("/deployments/new");
  const list = page.getByRole("listbox", { name: "选择应用" });
  await expect(list).toBeVisible();
  await expect(list.getByRole("option").first()).toContainText("应用 40");

  const metrics = await page.evaluate(() => {
    const main = document.querySelector(".main-column");
    const body = document.querySelector(".deployment-create__body");
    const applicationList = document.querySelector(".deployment-application-list");
    if (!main || !body || !applicationList) throw new Error("缺少部署页布局容器");
    return {
      mainScrolls: main.scrollHeight > main.clientHeight + 1,
      bodyScrolls: body.scrollHeight > body.clientHeight + 1,
      listScrolls: applicationList.scrollHeight > applicationList.clientHeight + 1,
      listHasOverflowY: getComputedStyle(applicationList).overflowY === "auto",
    };
  });

  expect(metrics.mainScrolls).toBe(false);
  expect(metrics.bodyScrolls).toBe(false);
  expect(metrics.listHasOverflowY).toBe(true);
  expect(metrics.listScrolls).toBe(true);
});

test("部署详情通过 axe smoke", async ({ page }) => {
  await authenticate(page);
  await page.route("**/api/v1/deployments/deployment-1", (route) => json(route, deployment));
  await page.route("**/api/v1/deployments/deployment-1/logs", (route) => route.fulfill({ contentType: "text/event-stream", body: "id: 1\nevent: log\ndata: {\"sequence\":1,\"stream\":\"stdout\",\"content\":\"safe output\",\"truncated\":false,\"created_at\":\"2026-08-02T00:00:02Z\"}\n\n" }));
  await page.goto("/deployments/deployment-1?view=logs");
  await expect(page.getByText("safe output")).toBeVisible();

  const results = await new AxeBuilder({ page }).analyze();

  expect(results.violations.filter((item) => ["serious", "critical"].includes(item.impact ?? ""))).toEqual([]);
});

test("大批量日志保持 1000 条窗口且主操作可用", async ({ page }) => {
  await authenticate(page);
  const body = Array.from({ length: 1100 }, (_, index) => {
    const sequence = index + 1;
    return `id: ${sequence}\nevent: log\ndata: ${JSON.stringify({ sequence, stream: "stdout", content: `line-${sequence}`, truncated: false, created_at: "2026-08-02T00:00:02Z" })}\n\n`;
  }).join("");
  await page.route("**/api/v1/deployments/deployment-1", (route) => json(route, deployment));
  await page.route("**/api/v1/deployments/deployment-1/logs", (route) => route.fulfill({ contentType: "text/event-stream", body }));
  await page.goto("/deployments/deployment-1?view=logs");
  await expect(page.getByText("line-1100")).toBeVisible();
  await expect(page.locator(".log-line")).toHaveCount(1000);

  await page.getByRole("button", { name: "暂停跟随" }).press("Enter");
  await expect(page.getByRole("button", { name: "恢复跟随" })).toBeVisible();
  await page.getByRole("button", { name: "取消部署" }).press("Enter");
  await expect(page.getByRole("dialog", { name: "取消部署" })).toBeVisible();
  await page.getByRole("button", { name: "返回" }).press("Enter");
});

test("执行中断时确认重试范围并使用稳定幂等键重试", async ({ page }) => {
  await authenticate(page);
  const interrupted = { ...deployment, id: "deployment-interrupted", status: "interrupted", phase: "interrupted", finished_at: "2026-08-02T00:05:00Z", target_runs: targetRuns.map((run) => ({ ...run, status: "failed", phase: "interrupted", error_code: "agent_disconnected", finished_at: "2026-08-02T00:05:00Z" })) };
  const retryKeys: string[] = [];
  await page.route("**/api/v1/deployments/deployment-interrupted", (route) => json(route, interrupted));
  await page.route("**/api/v1/deployments/deployment-interrupted/logs", (route) => route.fulfill({ contentType: "text/event-stream", body: "event: terminal\ndata: {\"status\":\"interrupted\",\"last_event_id\":0}\n\n" }));
  await page.route("**/api/v1/deployments/deployment-interrupted/retry", async (route) => {
    retryKeys.push(route.request().headers()["idempotency-key"] ?? "");
    await new Promise((resolve) => setTimeout(resolve, 30));
    await json(route, { ...deployment, id: "deployment-retry" }, 201);
  });
  await page.route("**/api/v1/deployments/deployment-retry", (route) => json(route, { ...deployment, id: "deployment-retry" }));
  await page.route("**/api/v1/deployments/deployment-retry/logs", (route) => route.fulfill({ contentType: "text/event-stream", body: "event: terminal\ndata: {\"status\":\"succeeded\",\"last_event_id\":0}\n\n" }));

  await page.goto("/deployments/deployment-interrupted");
  await expect(page.getByText(/无法证明远端脚本的最终状态/)).toBeVisible();
  await page.getByRole("button", { name: "重试失败目标" }).press("Enter");
  const retryDialog = page.getByRole("dialog", { name: "重试失败目标" });
  await expect(retryDialog.getByText("node-1")).toBeVisible();
  await expect(retryDialog.getByText("node-2")).toBeVisible();
  await retryDialog.getByRole("button", { name: "确认重试 2 个目标" }).dblclick();
  await expect(page).toHaveURL(/\/deployments\/deployment-retry$/);
  expect(retryKeys).toHaveLength(1);
  expect(retryKeys[0]).toMatch(/^retry-[0-9a-f-]{36}$/);
});
