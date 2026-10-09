import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { Select } from "../components/form";

function Fixture() {
  const [value, setValue] = useState("");
  return <><Select searchable value={value} searchPlaceholder="搜索应用" onChange={(event) => setValue(event.target.value)}><option value="">全部应用</option><option value="bi">BI 系统</option><option value="mall">独立商城</option></Select><button>外部按钮</button></>;
}

it("可搜索下拉保留输入焦点、空匹配、键盘选择与关闭", async () => {
  const user = userEvent.setup(); render(<Fixture />);
  const trigger = screen.getByRole("button", { name: "全部应用" });
  await user.click(trigger);
  const search = screen.getByRole("textbox", { name: "搜索应用" });
  await waitFor(() => expect(search).toHaveFocus());
  fireEvent.keyDown(search, { key: "Enter", isComposing: true });
  fireEvent.keyDown(search, { key: "ArrowDown", isComposing: true });
  fireEvent.keyDown(search, { key: "Enter", keyCode: 229 });
  fireEvent.keyDown(search, { key: "ArrowDown", keyCode: 229 });
  expect(search).toHaveFocus();
  expect(screen.getByRole("listbox")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "全部应用" })).toBeInTheDocument();
  await user.type(search, "none");
  expect(screen.getByRole("status")).toHaveTextContent("没有匹配的选项");
  await user.clear(search); await user.type(search, "bi");
  await user.keyboard("{ArrowDown}{Enter}");
  expect(screen.getByRole("button", { name: "BI 系统" })).toHaveFocus();
  expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "BI 系统" }));
  expect(screen.getByRole("textbox", { name: "搜索应用" })).toHaveValue("");
  await waitFor(() => expect(screen.getByRole("textbox", { name: "搜索应用" })).toHaveFocus());
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "BI 系统" }));
  await user.click(screen.getByRole("button", { name: "外部按钮" }));
  expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
});

it("非搜索下拉保持键盘选择行为", async () => {
  const changed = vi.fn(); const user = userEvent.setup();
  render(<Select value="first" onChange={changed}><option value="first">第一项</option><option value="second">第二项</option></Select>);
  await user.click(screen.getByRole("button", { name: "第一项" }));
  expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("option", { name: "第一项" })).toHaveFocus());
  await user.keyboard("{ArrowDown}");
  await waitFor(() => expect(screen.getByRole("option", { name: "第二项" })).toHaveFocus());
  await user.keyboard("{Enter}");
  expect(changed).toHaveBeenCalledWith({ target: { value: "second" } });
  expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
});
