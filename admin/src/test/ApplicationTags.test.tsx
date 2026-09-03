import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-tags", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const applications = [
  {
    id: "app-alpha",
    name: "Alpha 平台",
    slug: "alpha-platform",
    description: "",
    environment: "prod",
    status: "active",
    version: 1,
    tags: ["中间件", "项目A"],
    created_at: "2026-09-03T00:00:00Z",
    updated_at: "2026-09-03T00:00:00Z",
  },
  {
    id: "app-beta",
    name: "Beta 数据",
    slug: "beta-data",
    description: "",
    environment: "test",
    status: "active",
    version: 1,
    tags: ["项目B"],
    created_at: "2026-09-03T00:00:01Z",
    updated_at: "2026-09-03T00:00:01Z",
  },
];

function mockApplicationTags() {
  server.use(
    http.get("/api/v1/application-tags", () => HttpResponse.json({ tags: ["中间件", "项目A", "项目B"] })),
    http.get("/api/v1/applications", ({ request }) => {
      const url = new URL(request.url);
      const tag = url.searchParams.get("tag");
      const items = tag ? applications.filter((application) => application.tags.includes(tag)) : applications;
      return HttpResponse.json({ items, next_cursor: null });
    }),
  );
}

function renderAppsPage() {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: ["/apps"] });
  return render(<AppProviders initialAuth={administrator}><RouterProvider router={router} /></AppProviders>);
}

describe("应用标签", () => {
  it("点击标签筛选应用列表", async () => {
    mockApplicationTags();
    const user = userEvent.setup();
    renderAppsPage();
    expect(await screen.findByText("Alpha 平台")).toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: "配置" })).toHaveLength(2);
    expect(screen.getByText("Beta 数据")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "项目A" }));
    expect(await screen.findByText("Alpha 平台")).toBeInTheDocument();
    expect(screen.queryByText("Beta 数据")).not.toBeInTheDocument();
  });

  it("创建应用时可选择已有标签并新建标签", async () => {
    mockApplicationTags();
    let createBody: unknown;
    server.use(
      http.post("/api/v1/applications", async ({ request }) => {
        createBody = await request.json();
        return HttpResponse.json(
          {
            id: "app-new",
            name: "New Service",
            slug: "new-service",
            description: "",
            environment: "prod",
            app_type: "binary",
            type_version: "1",
            parameter_schema: {},
            verification_config: {},
            status: "active",
            version: 1,
            tags: ["中间件", "数据分析"],
            created_at: "2026-09-03T00:00:02Z",
            updated_at: "2026-09-03T00:00:02Z",
          },
          { status: 201 },
        );
      }),
    );
    const user = userEvent.setup();
    renderAppsPage();
    await user.click(await screen.findByRole("button", { name: "创建应用" }));
    await user.type(await screen.findByLabelText("应用名称"), "New Service");
    await user.type(screen.getByLabelText("Slug"), "new-service");
    const tagField = screen.getByText(/可点击已有标签或新建/).closest(".form-field") as HTMLElement | null;
    if (!tagField) throw new Error("标签表单未找到");
    await user.click(within(tagField).getByRole("button", { name: "中间件" }));
    await user.click(screen.getByRole("button", { name: "新建标签" }));
    await user.type(screen.getByLabelText("新标签名称"), "数据分析");
    await user.click(screen.getByRole("button", { name: "添加" }));
    await user.click(screen.getByRole("button", { name: "保存应用" }));
    await waitFor(() => {
      expect(createBody).toEqual(
        expect.objectContaining({
          name: "New Service",
          slug: "new-service",
          tags: ["中间件", "数据分析"],
        }),
      );
    });
  });
});
