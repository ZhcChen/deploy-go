import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import fixtureData from "../../../test-fixtures/client-behavior.json";
import { ApiErrorNotice } from "../features/errors/ApiErrorNotice";

interface ContractFixture {
  errors: Array<{ status: number; code: string; message: string; request_id: string }>;
  cursor: {
    pages: Array<{ items: string[]; next_cursor: string | null }>;
    expected_items: string[];
  };
}

const fixture = fixtureData as ContractFixture;

describe("跨端客户端行为契约", () => {
  it("覆盖统一错误状态且每项保留 Request ID", () => {
    expect(fixture.errors.map((item) => item.status)).toEqual([401, 403, 409, 422, 500]);
    expect(fixture.errors.every((item) => item.code && item.message && item.request_id)).toBe(true);
  });

  it("cursor 链按资源 ID 去重", () => {
    const items = [...new Set(fixture.cursor.pages.flatMap((page) => page.items))];
    expect(items).toEqual(fixture.cursor.expected_items);
  });

  it("Request ID 可通过可访问命令复制", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const conflict = fixture.errors.find((item) => item.status === 409)!;
    render(<ApiErrorNotice error={{ message: conflict.message, requestId: conflict.request_id }} />);

    await userEvent.click(screen.getByRole("button", { name: "复制 Request ID" }));

    expect(writeText).toHaveBeenCalledWith("req-fixture-409");
    expect(screen.getByText("已复制")).toBeVisible();
  });
});
