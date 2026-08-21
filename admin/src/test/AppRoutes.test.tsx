import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { AppProviders } from "../app/AppProviders";
import { Button } from "../components/Button";
import { PageState } from "../components/PageState";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { http, HttpResponse } from "msw";
import { server } from "./server";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";

const administrator: AuthSnapshot = {
  status: "authenticated",
  csrfToken: "csrf-for-test",
  user: { id: "admin-1", username: "admin", displayName: "陈舟", identity: "administrator" },
};

function mockOverviewEmpty() {
  server.use(
    http.get("/api/v1/deployments", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications", () => HttpResponse.json({ items: [], next_cursor: null })),
  );
}

function renderRoute(path: string) {
  let queryClient: QueryClient | undefined;
  const view = render(
    <MemoryRouter initialEntries={[path]}>
      <AppProviders initialAuth={administrator}>
        <QueryClientCapture onCapture={(client) => { queryClient = client; }} />
        <AppRoutes />
      </AppProviders>
    </MemoryRouter>,
  );
  if (!queryClient) throw new Error("QueryClient 未初始化");
  return { ...view, queryClient };
}

function QueryClientCapture({ onCapture }: { onCapture(client: QueryClient): void }) {
  onCapture(useQueryClient());
  return null;
}

describe("Web 路由壳", () => {
  it("支持设置二级路由深链并标记当前项", () => {
    renderRoute("/settings/audit");
    expect(screen.getByRole("heading", { level: 1, name: "审计记录" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "设置导航" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "审计记录" })).toHaveClass("is-active");
  });

  it("主导航可以切换页面", async () => {
    mockOverviewEmpty();
    const user = userEvent.setup();
    renderRoute("/overview");
    await user.click(screen.getByRole("link", { name: "节点" }));
    expect(screen.getByRole("heading", { level: 1, name: "节点" })).toBeInTheDocument();
  });

  it("部署记录按 cursor 翻页并可返回已缓存页面", async () => {
    const requests: Array<string | null> = [];
    const limits: Array<string | null> = [];
    server.use(http.get("/api/v1/deployments", ({ request }) => {
      const after = new URL(request.url).searchParams.get("after");
      requests.push(after);
      limits.push(new URL(request.url).searchParams.get("limit"));
      const item = after ? { id: "deployment-2", target_id: "target-2", application_id: "app-2", application_name: "API Service" } : { id: "deployment-1", target_id: "target-1", application_id: "app-1", application_name: "Voucher Hub" };
      return HttpResponse.json({ items: [{ ...item, status: "succeeded", phase: "completed", created_at: "2026-08-07T00:00:00Z", target_runs: [], stage_tasks: [] }], next_cursor: after ? null : "cursor-1" });
    }));
    const user = userEvent.setup();
    renderRoute("/deployments");

    expect(await screen.findByText("deployment-1")).toBeInTheDocument();
    expect(screen.getByText("Voucher Hub")).toBeInTheDocument();
    expect(screen.getByText("第 1 页")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "上一页" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "下一页" }));
    expect(await screen.findByText("deployment-2")).toBeInTheDocument();
    expect(screen.getByText("API Service")).toBeInTheDocument();
    expect(screen.getByText("第 2 页")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "下一页" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "上一页" }));
    expect(screen.getByText("deployment-1")).toBeInTheDocument();
    expect(requests).toEqual([null, "cursor-1"]);
    expect(limits).toEqual(["10", "10"]);
  });

  it("未知路由显示独立 404", () => {
    renderRoute("/missing-page");
    expect(screen.getByRole("heading", { name: "页面不存在" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "主导航" })).not.toBeInTheDocument();
  });

  it("普通用户不显示设置导航且直接访问返回 403", () => {
    render(
      <MemoryRouter initialEntries={["/settings/users"]}>
        <AppProviders initialAuth={{ ...administrator, user: { ...administrator.user!, identity: "user" } }}>
          <AppRoutes />
        </AppProviders>
      </MemoryRouter>,
    );
    expect(screen.queryByRole("link", { name: "设置" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
  });

  it("退出网络失败时保留当前身份并允许重试", async () => {
    mockOverviewEmpty();
    server.use(http.post("/api/v1/auth/logout", () => HttpResponse.error()));
    const user = userEvent.setup();
    renderRoute("/overview");
    await user.click(screen.getByRole("button", { name: "退出登录" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("退出失败，请重试");
    expect(screen.getByRole("navigation", { name: "主导航" })).toBeInTheDocument();
  });

  it("退出成功时清空上一身份的查询缓存", async () => {
    mockOverviewEmpty();
    server.use(http.post("/api/v1/auth/logout", () => new HttpResponse(null, { status: 204 })));
    const user = userEvent.setup();
    const { queryClient } = renderRoute("/overview");
    queryClient.setQueryData(["deployment", "protected"], { id: "protected" });

    await user.click(screen.getByRole("button", { name: "退出登录" }));

    expect(await screen.findByRole("heading", { name: "登录" })).toBeInTheDocument();
    expect(queryClient.getQueryCache().getAll()).toHaveLength(0);
  });
});

describe("基础组件状态", () => {
  it.each(["loading", "empty", "error"] as const)("%s 状态维持稳定工作区", (kind) => {
    const { container } = render(<PageState kind={kind} />);
    expect(container.firstChild).toHaveClass("page-state", `page-state--${kind}`);
  });

  it("危险色只由危险语义按钮启用", () => {
    render(<><Button>保存</Button><Button tone="danger">删除</Button></>);
    expect(screen.getByRole("button", { name: "保存" })).not.toHaveClass("button--danger");
    expect(screen.getByRole("button", { name: "删除" })).toHaveClass("button--danger");
  });
});
