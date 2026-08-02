import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { AppProviders } from "../app/AppProviders";
import { Button } from "../components/Button";
import { PageState } from "../components/PageState";
import { AppRoutes } from "../routes/AppRoutes";

function renderRoute(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AppProviders>
        <AppRoutes />
      </AppProviders>
    </MemoryRouter>,
  );
}

describe("Web 路由壳", () => {
  it("支持设置二级路由深链并标记当前项", () => {
    renderRoute("/settings/credentials");
    expect(screen.getByRole("heading", { level: 1, name: "SSH 凭证" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "设置导航" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "SSH 凭证" })).toHaveClass("is-active");
  });

  it("主导航可以切换页面", async () => {
    const user = userEvent.setup();
    renderRoute("/overview");
    await user.click(screen.getByRole("link", { name: "节点" }));
    expect(screen.getByRole("heading", { level: 1, name: "节点" })).toBeInTheDocument();
  });

  it("未知路由显示独立 404", () => {
    renderRoute("/missing-page");
    expect(screen.getByRole("heading", { name: "页面不存在" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "主导航" })).not.toBeInTheDocument();
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
