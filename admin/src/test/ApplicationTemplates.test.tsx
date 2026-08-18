import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-templates", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("应用模板", () => {
  it("展示模板列表并只读查看配置文件", async () => {
    const user = userEvent.setup();
    renderRoute("/templates");
    expect(await screen.findByRole("heading", { level: 2, name: "应用模板" })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "模板列表" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /PostgreSQL 18/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: /Redis 7/ })).toBeInTheDocument();
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("PostgreSQL 18");

    await user.click(screen.getByRole("tab", { name: /Redis 7/ }));
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("Redis 7");
    expect(screen.getByRole("link", { name: "从模板创建应用" })).toHaveAttribute("href", "/templates/new?template=redis");

    await user.click(screen.getByRole("tab", { name: /PostgreSQL 18/ }));
    await user.click(screen.getByRole("tab", { name: "Compose 编排" }));
    expect(screen.getByTestId("template-file-content")).toHaveTextContent("postgres:18-alpine");

    await user.click(screen.getByRole("tab", { name: "应用配置" }));
    expect(screen.getByTestId("template-file-content")).toHaveTextContent("max_connections = 100");
  });

  it("管理员看到从模板创建应用入口", () => {
    renderRoute("/templates");
    expect(screen.getByRole("link", { name: "从模板创建应用" })).toHaveAttribute("href", "/templates/new?template=postgres");
  });

  it("普通用户模板页不显示创建入口", () => {
    renderRoute("/templates", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(screen.getByRole("heading", { level: 2, name: "应用模板" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "从模板创建应用" })).not.toBeInTheDocument();
  });
});
