import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { THEME_STORAGE_KEY } from "../theme/ThemeContext";

const administrator: AuthSnapshot = {
  status: "authenticated",
  csrfToken: "csrf-for-test",
  user: { id: "admin-1", username: "admin", displayName: "陈舟", identity: "administrator" },
};

function renderAppearance() {
  return render(
    <MemoryRouter initialEntries={["/settings/appearance"]}>
      <AppProviders initialAuth={administrator}>
        <AppRoutes />
      </AppProviders>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  const values = new Map<string, string>();
  const storage: Storage = {
    get length() { return values.size; },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => { values.delete(key); },
    setItem: (key, value) => { values.set(key, value); },
  };
  Object.defineProperty(window, "localStorage", { configurable: true, value: storage });
});

afterEach(() => {
  window.localStorage.removeItem(THEME_STORAGE_KEY);
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.colorScheme = "";
});

describe("显示设置", () => {
  it("默认使用暗色主题并持久化亮色选择", async () => {
    const user = userEvent.setup();
    const view = renderAppearance();

    expect(await screen.findByRole("heading", { level: 2, name: "显示设置" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "暗色" })).toBeChecked();
    expect(document.documentElement.dataset.theme).toBe("dark");

    await user.click(screen.getByRole("radio", { name: "亮色" }));

    expect(screen.getByRole("radio", { name: "亮色" })).toBeChecked();
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.style.colorScheme).toBe("light");
    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe("light");

    view.unmount();
    renderAppearance();

    expect(screen.getByRole("radio", { name: "亮色" })).toBeChecked();
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("无效的本地主题值回退到暗色", () => {
    window.localStorage.setItem(THEME_STORAGE_KEY, "blue");

    renderAppearance();

    expect(screen.getByRole("radio", { name: "暗色" })).toBeChecked();
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
