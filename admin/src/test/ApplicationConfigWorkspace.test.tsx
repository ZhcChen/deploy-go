import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-config", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const composeFile = {
  id: "cfg-compose",
  application_id: "app-1",
  binding_id: "binding-compose",
  current_version: 1,
  delivery: "artifact",
  deploy_path: "compose.yaml",
  description: "Valkey Compose 配置",
  editable: true,
  format: "yaml",
  label: "compose.yaml",
  language: "yaml",
  path: "compose.yaml",
  recommended_changes: "",
  role: "application",
  sensitive: false,
  status: "active",
  updated_at: "2026-08-20T00:00:00Z",
  version: 1,
};
const valkeyEnvFile = {
  ...composeFile,
  id: "cfg-valkey-env",
  binding_id: "binding-valkey-env",
  deploy_path: "valkey.env",
  description: "Valkey 运行配置",
  format: "dotenv",
  label: "valkey.env",
  language: "properties",
  path: "valkey.env",
  sensitive: true,
};

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

describe("应用配置工作区", () => {
  it("点击敏感配置文件时切换选中文件并显示重新验证面板", async () => {
    server.use(
      http.get("/api/v1/applications/app-1/config-files", () => HttpResponse.json({ items: [composeFile, valkeyEnvFile], next_cursor: null })),
      http.get("/api/v1/application-config-files/cfg-compose", () => HttpResponse.json({ ...composeFile, content: "services:\n  valkey:\n    image: valkey/valkey:9-alpine\n" })),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config");

    expect(await screen.findByRole("textbox", { name: "compose.yaml 编辑器" })).toBeInTheDocument();
    const sensitiveFile = screen.getByRole("button", { name: /valkey\.env/ });
    expect(sensitiveFile).toHaveAttribute("aria-selected", "false");

    await user.click(sensitiveFile);

    expect(sensitiveFile).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("heading", { name: "重新验证管理员密码" })).toBeInTheDocument();
    expect(screen.getByLabelText("管理员密码")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "验证并继续" })).toBeInTheDocument();
  });

  it("重新验证后读取敏感配置明文并可编辑", async () => {
    server.use(
      http.get("/api/v1/applications/app-1/config-files", () => HttpResponse.json({ items: [composeFile, valkeyEnvFile], next_cursor: null })),
      http.get("/api/v1/application-config-files/cfg-compose", () => HttpResponse.json({ ...composeFile, content: "services:\n  valkey:\n    image: valkey/valkey:9-alpine\n" })),
      http.post("/api/v1/applications/app-1/config-reveal-grants", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-config");
        expect(await request.json()).toEqual({ action: "read_write", password: "correct-password" });
        return HttpResponse.json({ action: "read_write", grant_token: "grant-config", expires_at: "2099-08-20T00:05:00Z" });
      }),
      http.get("/api/v1/application-config-files/cfg-valkey-env", ({ request }) => {
        expect(request.headers.get("X-Env-Reveal-Grant")).toBe("grant-config");
        return HttpResponse.json({ ...valkeyEnvFile, content: "VALKEY_PASSWORD=preset-password\n" });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1/config");

    await user.click(await screen.findByRole("button", { name: /valkey\.env/ }));
    await user.type(await screen.findByLabelText("管理员密码"), "correct-password");
    await user.click(screen.getByRole("button", { name: "验证并继续" }));

    const editor = await screen.findByRole("textbox", { name: "valkey.env 编辑器" });
    expect(editor).toHaveTextContent("VALKEY_PASSWORD=preset-password");
    expect(screen.getByText(/敏感配置授权有效至/)).toBeInTheDocument();
  });
});
