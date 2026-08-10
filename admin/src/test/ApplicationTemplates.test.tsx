import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-templates", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };

function renderRoute(path: string) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={administrator}><RouterProvider router={router} /></AppProviders>);
}

describe("应用模板", () => {
  it("展示模板列表并只读查看配置文件", async () => {
    const user = userEvent.setup();
    renderRoute("/templates");
    expect(await screen.findByRole("heading", { level: 2, name: "应用模板" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /PostgreSQL 16/ })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /Redis 7/ })).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: /PostgreSQL 16/ }));
    await user.click(screen.getByRole("tab", { name: "Compose 编排" }));
    expect(screen.getByTestId("template-file-content")).toHaveTextContent("postgres:16-alpine");

    await user.click(screen.getByRole("tab", { name: "应用配置" }));
    expect(screen.getByTestId("template-file-content")).toHaveTextContent("max_connections = 100");
  });
});
