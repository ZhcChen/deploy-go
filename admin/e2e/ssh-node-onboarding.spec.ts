import { expect, test, type Page, type Route } from "@playwright/test";

const admin = { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" };
const credential = { id: "cred-1", name: "生产密钥", algorithm: "ed25519", public_key: "ssh-ed25519 AAAAFIXTURE deploy-go", fingerprint: "SHA256:credential", version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };
let node = { id: "node-1", name: "生产节点", host: "node.fixture.invalid", port: 22, username: "deploy", ssh_credential_id: "cred-1", work_root: "/srv/apps", secrets_root: "/srv/secrets", status: "unchecked", trusted_host_fingerprint: null as string | null, checked_at: null, version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" };

async function json(route: Route, body: unknown, status = 200) { await route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) }); }
async function authenticatedApi(page: Page) {
  await page.route("**/api/v1/setup", (route) => json(route, { setup_required: false, setup_enabled: false }));
  await page.route("**/api/v1/auth/me", (route) => json(route, admin));
  await page.route("**/api/v1/auth/csrf", (route) => json(route, { csrf_token: "csrf-onboarding" }));
  await page.route("**/api/v1/ssh-credentials", (route) => json(route, { items: [credential], next_cursor: null }));
  await page.route("**/api/v1/nodes/node-1", (route) => json(route, node));
}

test("管理员完成扫描、确认和节点检查", async ({ page }) => {
  node = { ...node, status: "unchecked", trusted_host_fingerprint: null, version: 1 };
  await authenticatedApi(page);
  await page.route("**/api/v1/nodes/node-1/host-key/scan", async (route) => {
    expect(route.request().headers()["x-csrf-token"]).toBe("csrf-onboarding");
    await json(route, { check_id: "scan-1", fingerprint: "SHA256:host-key", snapshot_hash: "snapshot-1" }, 201);
  });
  await page.route("**/api/v1/nodes/node-1/host-key/confirm", async (route) => {
    expect(await route.request().postDataJSON()).toEqual({ check_id: "scan-1", snapshot_hash: "snapshot-1", version: 1 });
    node = { ...node, trusted_host_fingerprint: "SHA256:host-key", version: 2 };
    await json(route, node);
  });
  await page.route("**/api/v1/nodes/node-1/checks", (route) => json(route, { id: "check-1", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 21474836480, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, 201));
  await page.goto("/nodes/node-1");
  await expect(page.getByRole("button", { name: "执行检查" })).toBeDisabled();
  await page.getByRole("button", { name: "扫描指纹" }).click();
  await expect(page.getByText("SHA256:host-key")).toBeVisible();
  await page.getByRole("button", { name: "确认指纹一致" }).click();
  await expect(page.getByRole("button", { name: "执行检查" })).toBeEnabled();
  await page.getByRole("button", { name: "执行检查" }).click();
  await expect(page.getByText("20.0 GiB")).toBeVisible();
});
