import { act, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { useUnsavedChanges } from "../features/shared/useUnsavedChanges";

const message = "当前页面有未保存的修改，确定离开吗？";

function Draft({ dirty }: { dirty: boolean }) {
  useUnsavedChanges(dirty);
  return <p>编辑中</p>;
}

function renderDraft(dirty: boolean) {
  const router = createMemoryRouter(
    [
      { path: "/", element: <Draft dirty={dirty} /> },
      { path: "/other", element: <p>其他页面</p> },
    ],
    { initialEntries: ["/"] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

describe("未保存修改的离开保护", () => {
  it("关闭或刷新标签页时不拦截，不触发浏览器的离开确认", () => {
    renderDraft(true);
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    // 浏览器只有在监听器调用了 preventDefault 时才会弹出原生的离开确认。
    expect(event.defaultPrevented).toBe(false);
  });

  it("应用内切换页面前仍需确认：取消则留在当前页，确定则离开", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);
    const router = renderDraft(true);

    await act(async () => { await router.navigate("/other"); });
    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(confirm).toHaveBeenCalledWith(message);
    expect(router.state.location.pathname).toBe("/");
    expect(screen.getByText("编辑中")).toBeInTheDocument();

    await act(async () => { await router.navigate("/other"); });
    await waitFor(() => expect(router.state.location.pathname).toBe("/other"));
    expect(confirm).toHaveBeenCalledTimes(2);
    expect(await screen.findByText("其他页面")).toBeInTheDocument();
  });

  it("没有未保存的修改时不拦截任何离开", async () => {
    const confirm = vi.spyOn(window, "confirm");
    const router = renderDraft(false);

    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);

    await act(async () => { await router.navigate("/other"); });
    await waitFor(() => expect(router.state.location.pathname).toBe("/other"));
    expect(confirm).not.toHaveBeenCalled();
  });
});
