import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-source", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const operator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-source", user: { id: "user-1", username: "operator", displayName: "部署用户", identity: "user" } };
const application = { id: "app-1", name: "Voucher Hub", slug: "voucher-hub", description: "代金券服务", environment: "prod", status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const credential = { id: "cred-1", name: "voucher-hub read key", algorithm: "ed25519", public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEXAMPLE", fingerprint: "SHA256:credential-fingerprint", status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" };
const agent = { id: "agent-1", name: "Build Agent", node_id: "node-1", environment: "测试", status: "online", protocol_version: 11, agent_version: "0.1.0", created_at: "2026-08-02T00:00:00Z" };
const sourceMissing = { code: "not_found", message: "应用来源不存在", request_id: "req-source-missing" };
const commitSha = "0123456789abcdef0123456789abcdef01234567";
const draftSource = {
  id: "source-1",
  application_id: "app-1",
  repository_url: "git@github.com:org/voucher-hub.git",
  git_credential_id: "cred-1",
  git_credential_name: "voucher-hub read key",
  build_agent_id: "agent-1",
  build_agent_name: "Build Agent",
  source_policy: "branch",
  status: "draft",
  deployment_branch: null,
  branch_verified_at: null,
  version: 1,
  created_at: "2026-08-02T00:00:00Z",
  updated_at: "2026-08-02T00:00:00Z",
};
const verifiedSource = {
  ...draftSource,
  status: "verified",
  deployment_branch: "main",
  branch_verified_at: "2026-08-02T01:00:00Z",
  version: 2,
  updated_at: "2026-08-02T01:00:00Z",
};
const refs = [
  { name: "main", ref: "refs/heads/main", sha: commitSha },
  { name: "develop", ref: "refs/heads/develop", sha: "abcdefabcdefabcdefabcdefabcdefabcdefabcd" },
];
const queuedDiscovery = {
  id: "ref-1",
  application_source_id: "source-1",
  task_id: "task-refs-1",
  status: "queued",
  source_version: 1,
  refs: [],
  created_at: "2026-08-02T00:30:00Z",
  finished_at: null,
  expires_at: null,
};
const succeededDiscovery = {
  ...queuedDiscovery,
  status: "succeeded",
  refs,
  finished_at: "2026-08-02T00:30:01Z",
  expires_at: "2026-08-02T00:40:00Z",
};
function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

function baseHandlers() {
  return [
    http.get("/api/v1/applications/app-1", () => HttpResponse.json(application)),
    http.get("/api/v1/applications/app-1/targets", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/applications/app-1/env-files", () => HttpResponse.json({ items: [], next_cursor: null })),
    http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
  ];
}

describe("Git 来源配置", () => {
  it("管理员保存来源、轮询 refs 并固定分支", async () => {
    let saveBody: unknown;
    let branchBody: unknown;
    let refreshCalls = 0;
    server.use(
      ...baseHandlers(),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json(sourceMissing, { status: 404 })),
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.put("/api/v1/applications/app-1/source", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-source");
        saveBody = await request.json();
        return HttpResponse.json(draftSource);
      }),
      http.post("/api/v1/applications/app-1/source/refreshes", ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-source");
        refreshCalls += 1;
        return HttpResponse.json(queuedDiscovery);
      }),
      http.get("/api/v1/applications/app-1/source/refreshes/ref-1", () => HttpResponse.json(succeededDiscovery)),
      http.put("/api/v1/applications/app-1/source/branch", async ({ request }) => {
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-source");
        branchBody = await request.json();
        return HttpResponse.json(verifiedSource);
      }),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");

    expect(await screen.findByRole("button", { name: "开始配置" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "配置来源" })).not.toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "开始配置工作区" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "配置工作区来源" })).not.toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "开始配置" }));
    await user.type(await screen.findByLabelText("仓库地址"), "git@github.com:org/voucher-hub.git");
    await user.click(await screen.findByLabelText("Git 凭证"));
    await user.click(await screen.findByRole("option", { name: "voucher-hub read key" }));
    await user.click(await screen.findByLabelText("构建节点"));
    await user.click(await screen.findByRole("option", { name: /Build Agent · v0\.1\.0/ }));
    const sourceForm = screen.getByLabelText("仓库地址").closest("form");
    if (!sourceForm) throw new Error("来源表单未渲染");
    fireEvent.submit(sourceForm);

    expect(await screen.findByRole("button", { name: "刷新分支" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "刷新分支" }));

    const branchSelect = await screen.findByLabelText("固定分支", {}, { timeout: 3000 });
    await user.click(branchSelect);
    await user.click(await screen.findByRole("option", { name: /main · 0123456789/ }));
    await user.click(screen.getByRole("button", { name: "固定分支并完成配置" }));

    expect(await screen.findByText("已验证")).toBeInTheDocument();
    expect(screen.getByText("git@github.com:org/voucher-hub.git")).toBeInTheDocument();
    expect(screen.getByText("main")).toBeInTheDocument();
    expect(screen.getByText("Build Agent")).toBeInTheDocument();
    expect(screen.getByText("voucher-hub read key")).toBeInTheDocument();
    expect(refreshCalls).toBe(1);
    expect(saveBody).toEqual({
      repository_url: "git@github.com:org/voucher-hub.git",
      git_credential_id: "cred-1",
      build_agent_id: "agent-1",
      source_policy: "branch",
    });
    expect(branchBody).toEqual({ branch: "main", version: 1 });
  });

  it("普通用户只能查看已固定来源，不能进入编辑", async () => {
    let credentialCalls = 0;
    let agentCalls = 0;
    server.use(
      ...baseHandlers(),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json(verifiedSource)),
      http.get("/api/v1/git-credentials", () => { credentialCalls += 1; return HttpResponse.json({ items: [], next_cursor: null }); }),
      http.get("/api/v1/agents", () => { agentCalls += 1; return HttpResponse.json({ items: [], next_cursor: null }); }),
    );
    renderRoute("/apps/app-1", operator);

    expect(await screen.findByText("git@github.com:org/voucher-hub.git")).toBeInTheDocument();
    expect(screen.getByText("main")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "配置来源" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "开始配置" })).not.toBeInTheDocument();
    expect(credentialCalls).toBe(0);
    expect(agentCalls).toBe(0);
  });

  it("编辑来源后旧分支发现立即失效", async () => {
    server.use(
      ...baseHandlers(),
      http.get("/api/v1/applications/app-1/source", () => HttpResponse.json(draftSource)),
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.post("/api/v1/applications/app-1/source/refreshes", () => HttpResponse.json(succeededDiscovery)),
      http.get("/api/v1/applications/app-1/source/refreshes/ref-1", () => HttpResponse.json(succeededDiscovery)),
    );
    const user = userEvent.setup();
    renderRoute("/apps/app-1");

    await user.click(await screen.findByRole("button", { name: "配置来源" }));
    await user.click(screen.getByRole("button", { name: "刷新分支" }));
    await screen.findByLabelText("固定分支", {}, { timeout: 3000 });

    await user.type(screen.getByLabelText("仓库地址"), "/voucher-hub.git");
    expect(screen.queryByLabelText("固定分支")).not.toBeInTheDocument();
    expect(screen.queryByText("固定分支并完成配置")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存来源" })).toBeEnabled();
  });
});
