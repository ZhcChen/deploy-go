import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

for (const width of [1280, 390]) {
  test(`部署记录应用搜索与联合筛选 ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    const requests: URL[] = [];
    await page.route("**/api/v1/setup", (route) => route.fulfill({ json: { setup_required: false } }));
    await page.route("**/api/v1/auth/me", (route) => route.fulfill({ json: { id: "admin-1", username: "admin", display_name: "管理员", identity: "administrator" } }));
    await page.route("**/api/v1/auth/csrf", (route) => route.fulfill({ json: { csrf_token: "test-csrf" } }));
    await page.route("**/api/v1/applications?**", (route) => route.fulfill({ json: { items: [{ id: "app-mall", name: "独立商城【测试环境】" }, { id: "app-bi", name: "BI系统【测试环境】" }], next_cursor: null } }));
    await page.route("**/api/v1/deployments?**", (route) => {
      const url = new URL(route.request().url());
      requests.push(url);
      return route.fulfill({ json: { items: [{ id: "deployment-filter", application_id: url.searchParams.get("application_id") || "app-mall", application_name: url.searchParams.has("application_id") ? "BI系统【测试环境】" : "独立商城【测试环境】", status: url.searchParams.get("status") || "succeeded", phase: "completed", created_at: "2026-10-09T00:00:00Z", target_runs: [], stage_tasks: [] }], next_cursor: url.searchParams.has("after") ? null : "next" } });
    });
    await page.goto("/deployments");
    await page.getByRole("button", { name: "下一页" }).click();
    await expect(page.getByText("第 2 页")).toBeVisible();
    await page.getByRole("button", { name: "应用", exact: true }).click();
    const search = page.getByRole("textbox", { name: "搜索应用" });
    await expect(search).toBeFocused();
    await search.fill("bi");
    await expect(page.getByRole("option")).toHaveCount(1);
    expect((await new AxeBuilder({ page }).include(".filter-bar").analyze()).violations).toEqual([]);
    await search.press("ArrowDown");
    await page.getByRole("option", { name: "BI系统【测试环境】" }).press("Enter");
    await expect(page.getByText("第 1 页")).toBeVisible();
    await expect.poll(() => requests.at(-1)?.searchParams.get("application_id")).toBe("app-bi");
    expect(requests.at(-1)?.searchParams.has("after")).toBe(false);
    await page.getByRole("button", { name: "状态", exact: true }).click();
    await page.getByRole("option", { name: "失败", exact: true }).click();
    await expect.poll(() => requests.at(-1)?.searchParams.get("status")).toBe("failed");
    expect(requests.at(-1)?.searchParams.get("application_id")).toBe("app-bi");
    await page.getByRole("button", { name: "应用", exact: true }).click();
    await page.getByRole("option", { name: "全部应用" }).click();
    await expect.poll(() => requests.at(-1)?.searchParams.has("application_id")).toBe(false);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
}
