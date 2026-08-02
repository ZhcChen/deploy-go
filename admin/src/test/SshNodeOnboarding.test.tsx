import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = {
  status: "authenticated",
  csrfToken: "csrf-onboarding",
  user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" },
};

const credential = {
  id: "cred-1", name: "生产密钥", algorithm: "ed25519",
  public_key: "ssh-ed25519 AAAATEST deploy-go", fingerprint: "SHA256:credential",
  version: 1, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z",
};

const baseNode = {
  id: "node-1", name: "生产节点", host: "node.fixture.invalid", port: 22, username: "deploy",
  ssh_credential_id: "cred-1", work_root: "/srv/apps", secrets_root: "/srv/secrets",
  status: "unchecked", trusted_host_fingerprint: null as string | null, checked_at: null as string | null, version: 1,
  created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z",
};

function renderRoute(path: string) {
  return render(<MemoryRouter initialEntries={[path]}><AppProviders initialAuth={administrator}><AppRoutes /></AppProviders></MemoryRouter>);
}

describe("SSH 密钥管理", () => {
  it("列表错误显示服务端消息和 request ID 并允许重试", async () => {
    server.use(http.get("/api/v1/ssh-credentials", () => HttpResponse.json({ code: "temporary_failure", message: "密钥存储暂不可用", request_id: "req-credential-list" }, { status: 500 })));
    renderRoute("/settings/credentials");
    expect(await screen.findByRole("alert", undefined, { timeout: 3000 })).toHaveTextContent("密钥存储暂不可用");
    expect(screen.getByText("Request ID: req-credential-list")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeEnabled();
  });

  it("复制失败时显示可选中的手动复制回退", async () => {
    server.use(
      http.get("/api/v1/ssh-credentials/cred-1", () => HttpResponse.json(credential)),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
    );
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: () => Promise.reject(new Error("denied")) } });
    renderRoute("/settings/credentials/cred-1");
    await user.click(await screen.findByRole("button", { name: "复制公钥" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("手动复制");
    expect(screen.getByText(credential.public_key)).toBeInTheDocument();
  });

  it("删除前重新读取绑定关系并阻止请求", async () => {
    let listCalls = 0;
    let deleteCalls = 0;
    server.use(
      http.get("/api/v1/ssh-credentials/cred-1", () => HttpResponse.json(credential)),
      http.get("/api/v1/nodes", () => {
        listCalls += 1;
        return HttpResponse.json({ items: listCalls === 1 ? [] : [baseNode], next_cursor: null });
      }),
      http.delete("/api/v1/ssh-credentials/cred-1", () => { deleteCalls += 1; return new HttpResponse(null, { status: 204 }); }),
    );
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    renderRoute("/settings/credentials/cred-1");
    await user.click(await screen.findByRole("button", { name: "删除" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("仍绑定节点");
    expect(listCalls).toBe(2);
    expect(deleteCalls).toBe(0);
  });
});

describe("节点接入", () => {
  it("扫描、人工确认和检查保持独立并使用最新扫描快照", async () => {
    let scanCount = 0;
    const calls: string[] = [];
    let node = { ...baseNode };
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(node)),
      http.get("/api/v1/ssh-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.post("/api/v1/nodes/node-1/host-key/scan", () => {
        scanCount += 1;
        calls.push(`scan-${scanCount}`);
        return HttpResponse.json({ check_id: `check-${scanCount}`, fingerprint: `SHA256:host-${scanCount}`, snapshot_hash: `snapshot-${scanCount}` }, { status: 201 });
      }),
      http.post("/api/v1/nodes/node-1/host-key/confirm", async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        calls.push(`confirm-${String(body.snapshot_hash)}`);
        node = { ...node, trusted_host_fingerprint: "SHA256:host-2", version: 2 };
        return HttpResponse.json(node);
      }),
      http.post("/api/v1/nodes/node-1/checks", () => {
        calls.push("check");
        return HttpResponse.json({ id: "check-result", status: "succeeded", os_name: "Linux", architecture: "x86_64", disk_available_bytes: 10737418240, created_at: "2026-08-01T00:00:00Z", finished_at: "2026-08-01T00:00:01Z" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    renderRoute("/nodes/node-1");
    const scanButton = await screen.findByRole("button", { name: "扫描指纹" });
    expect(screen.getByRole("button", { name: "执行检查" })).toBeDisabled();
    await user.click(scanButton);
    expect(await screen.findByText("SHA256:host-1")).toBeInTheDocument();
    expect(calls).toEqual(["scan-1"]);
    await user.click(scanButton);
    expect(await screen.findByText("SHA256:host-2")).toBeInTheDocument();
    expect(screen.queryByText("SHA256:host-1")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "确认指纹一致" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "执行检查" })).toBeEnabled());
    expect(calls).toEqual(["scan-1", "scan-2", "confirm-snapshot-2"]);
    await user.click(screen.getByRole("button", { name: "扫描指纹" }));
    expect(await screen.findByText("SHA256:host-3")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "执行检查" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "确认指纹一致" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "执行检查" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "执行检查" }));
    expect(await screen.findByText("10.0 GiB")).toBeInTheDocument();
    expect(calls.at(-1)).toBe("check");
  });

  it("普通用户可查看授权节点但不加载或显示接入管理", async () => {
    let credentialCalls = 0;
    server.use(
      http.get("/api/v1/nodes/node-1", () => HttpResponse.json(baseNode)),
      http.get("/api/v1/ssh-credentials", () => { credentialCalls += 1; return HttpResponse.json({ items: [credential] }); }),
    );
    render(<MemoryRouter initialEntries={["/nodes/node-1"]}><AppProviders initialAuth={{ ...administrator, user: { ...administrator.user!, identity: "user" } }}><AppRoutes /></AppProviders></MemoryRouter>);
    expect(await screen.findByRole("heading", { name: "生产节点" })).toBeInTheDocument();
    expect(screen.getByText("接入配置由管理员维护。", { exact: false })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "扫描指纹" })).not.toBeInTheDocument();
    expect(credentialCalls).toBe(0);
  });
});
