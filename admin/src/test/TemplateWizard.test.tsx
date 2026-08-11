// 从模板创建应用向导的集成测试
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { AppProviders } from "../app/AppProviders";
import { AppRoutes } from "../routes/AppRoutes";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-wizard", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const credential = {
  id: "cred-1",
  name: "deploy read key",
  algorithm: "ed25519",
  public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEXAMPLE",
  fingerprint: "SHA256:credential-fingerprint",
  status: "active",
  version: 1,
  created_at: "2026-08-02T00:00:00Z",
  updated_at: "2026-08-02T00:00:00Z",
};
const agent = {
  id: "agent-1",
  name: "Build Agent",
  node_id: "node-1",
  environment: "生产",
  status: "online",
  protocol_version: 7,
  agent_version: "0.2.0",
  created_at: "2026-08-02T00:00:00Z",
};
const node = {
  id: "node-1",
  name: "生产节点01",
  host: "node.fixture.invalid",
  work_root: "/srv/apps",
  privileged_execution: true,
  status: "online",
  version: 1,
  created_at: "2026-08-02T00:00:00Z",
  updated_at: "2026-08-02T00:00:00Z",
};
const draftSource = {
  id: "source-1",
  application_id: "app-wizard",
  repository_url: "git@github.com:org/pg-test.git",
  git_credential_id: "cred-1",
  git_credential_name: "deploy read key",
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
  refs: [{ name: "main", ref: "refs/heads/main", sha: "0123456789abcdef0123456789abcdef01234567" }],
  finished_at: "2026-08-02T00:30:01Z",
  expires_at: "2026-08-02T00:40:00Z",
};

function renderRoute(path: string, snapshot = administrator) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={snapshot}><RouterProvider router={router} /></AppProviders>);
}

async function createApp(user: ReturnType<typeof userEvent.setup>, name = "PG Test") {
  renderRoute("/templates/new?template=postgres");
  await user.click(await screen.findByRole("button", { name: "使用 PostgreSQL 18 继续" }));
  await user.clear(screen.getByLabelText("应用名称"));
  await user.type(screen.getByLabelText("应用名称"), name);
  await user.clear(screen.getByLabelText("Slug"));
  await user.type(screen.getByLabelText("Slug"), "pg-test");
  await user.click(screen.getByRole("button", { name: "创建应用并继续" }));
  expect(await screen.findByRole("heading", { name: "Git 来源与固定分支" })).toBeInTheDocument();
}

async function fillAndSubmitSource(user: ReturnType<typeof userEvent.setup>) {
  await waitFor(() => expect(screen.queryByText("正在加载凭证...")).not.toBeInTheDocument());
  await user.type(screen.getByLabelText("仓库地址"), "git@github.com:org/pg-test.git");
  await user.click(screen.getByLabelText("Git 凭证"));
  await user.click(await screen.findByRole("option", { name: "deploy read key" }));
  await user.click(screen.getByLabelText("构建节点"));
  await user.click(await screen.findByRole("option", { name: /Build Agent · v0\.2\.0/ }));
  await user.click(screen.getByRole("button", { name: "保存来源并扫描分支" }));
}

async function fixMainBranch(user: ReturnType<typeof userEvent.setup>) {
  const branchSelect = await screen.findByLabelText("固定分支", {}, { timeout: 3000 });
  await user.click(branchSelect);
  await user.click(await screen.findByRole("option", { name: /main · 0123456789/ }));
  await user.click(screen.getByRole("button", { name: "固定分支并继续" }));
  expect(await screen.findByRole("heading", { name: "部署目标" })).toBeInTheDocument();
}

describe("从模板创建应用向导", () => {
  it("管理员完整创建应用、固定分支并创建两阶段目标", async () => {
    const requests: string[] = [];
    let appBody: Record<string, unknown> | undefined;
    let sourceBody: Record<string, unknown> | undefined;
    let branchBody: Record<string, unknown> | undefined;
    let targetBody: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [node], next_cursor: null })),
      http.post("/api/v1/applications", async ({ request }) => {
        requests.push("applications.create");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        appBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "app-wizard", name: appBody.name, slug: appBody.slug, description: appBody.description, status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }, { status: 201 });
      }),
      http.put("/api/v1/applications/app-wizard/source", async ({ request }) => {
        requests.push("source.save");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        sourceBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json(draftSource);
      }),
      http.post("/api/v1/applications/app-wizard/source/refreshes", async ({ request }) => {
        requests.push("source.refresh");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        return HttpResponse.json(queuedDiscovery);
      }),
      http.get("/api/v1/applications/app-wizard/source/refreshes/ref-1", () => {
        requests.push("source.refresh.show");
        return HttpResponse.json(succeededDiscovery);
      }),
      http.put("/api/v1/applications/app-wizard/source/branch", async ({ request }) => {
        requests.push("source.branch");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        branchBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json(verifiedSource);
      }),
      http.post("/api/v1/applications/app-wizard/targets", async ({ request }) => {
        requests.push("targets.create");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        targetBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json({
          id: "target-1",
          application_id: "app-wizard",
          node_id: targetBody.node_id,
          environment: "production",
          execution_mode: targetBody.execution_mode,
          script_path: targetBody.script_path,
          parameter_schema: targetBody.parameter_schema,
          secret_file_references: [],
          verification_config: targetBody.verification_config,
          timeout_seconds: targetBody.timeout_seconds,
          privileged_release: targetBody.privileged_release,
          status: "active",
          snapshot_hash: "snap-1",
          version: 1,
          created_at: "2026-08-02T02:00:00Z",
          updated_at: "2026-08-02T02:00:00Z",
        }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    await createApp(user);
    await fillAndSubmitSource(user);
    await fixMainBranch(user);

    await user.click(screen.getByLabelText("节点"));
    await user.click(await screen.findByRole("option", { name: /生产节点01 · node\.fixture\.invalid/ }));
    await user.click(screen.getByRole("checkbox", { name: /使用 Agent 原生特权 release/ }));
    await user.click(screen.getByRole("button", { name: "创建目标" }));
    expect(await screen.findByText("开启 Agent 原生特权 release 前必须确认 root 信任边界")).toBeInTheDocument();
    expect(targetBody).toBeUndefined();

    await user.click(screen.getByRole("checkbox", { name: /我确认该仓库和固定分支的写入者将获得目标节点 root 发布能力/ }));
    await user.click(screen.getByRole("button", { name: "创建目标" }));
    expect(await screen.findByRole("heading", { name: "应用与部署目标已创建" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /PG Test/ })).toHaveAttribute("href", "/apps/app-wizard");

    expect(requests).toEqual(["applications.create", "source.save", "source.refresh", "source.refresh.show", "source.branch", "targets.create"]);
    expect(appBody).toMatchObject({ name: "PG Test", slug: "pg-test" });
    expect(sourceBody).toMatchObject({ repository_url: "git@github.com:org/pg-test.git", git_credential_id: "cred-1", build_agent_id: "agent-1", source_policy: "branch" });
    expect(branchBody).toEqual({ branch: "main", version: 1 });
    expect(targetBody).toMatchObject({
      node_id: "node-1",
      execution_mode: "two_stage",
      script_path: "/srv/apps/pg-test/placeholder",
      timeout_seconds: 900,
      privileged_release: true,
      privileged_release_confirmed: true,
      verification_config: { type: "tcp", port: 5432, timeout_ms: 5000 },
    });
    expect(targetBody).not.toHaveProperty("secret_file_references");
    expect((targetBody!.parameter_schema as { properties: Record<string, unknown> }).properties).toHaveProperty("release-version");
  });

  it("仅创建应用时不会请求来源与目标接口", async () => {
    const requests: string[] = [];
    server.use(
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.post("/api/v1/applications", async ({ request }) => {
        requests.push("applications.create");
        const body = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "app-wizard", name: body.name, slug: body.slug, description: body.description, status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    await createApp(user);
    await user.click(screen.getByRole("button", { name: "仅创建应用" }));

    expect(await screen.findByRole("heading", { name: "应用已创建" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "应用与部署目标已创建" })).not.toBeInTheDocument();
    expect(requests).toEqual(["applications.create"]);
  });

  it("镜像直连模式无需 Git 来源并创建特权镜像目标", async () => {
    const requests: string[] = [];
    let targetBody: Record<string, unknown> | undefined;
    server.use(
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [node], next_cursor: null })),
      http.get("/api/v1/applications/app-wizard/env-files", () => HttpResponse.json({ items: [
        { id: "env-compose", file_name: "compose.env", module: "compose", format: "dotenv-v1", current_version: 1, current_digest: "a".repeat(64), declared_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z", target_count: 0, pending_count: 0, syncing_count: 0, succeeded_count: 0, failed_count: 0, syncs: [] },
        { id: "env-postgres", file_name: "postgres.env", module: "postgres", format: "dotenv-v1", current_version: 1, current_digest: "a".repeat(64), declared_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z", target_count: 0, pending_count: 0, syncing_count: 0, succeeded_count: 0, failed_count: 0, syncs: [] },
      ], next_cursor: null })),
      http.post("/api/v1/applications", async ({ request }) => {
        requests.push("applications.create");
        const body = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "app-wizard", name: body.name, slug: body.slug, description: body.description, status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }, { status: 201 });
      }),
      http.post("/api/v1/applications/app-wizard/targets", async ({ request }) => {
        requests.push("targets.create");
        expect(request.headers.get("X-CSRF-Token")).toBe("csrf-wizard");
        targetBody = await request.json() as Record<string, unknown>;
        return HttpResponse.json({
          id: "target-image-1",
          application_id: "app-wizard",
          node_id: targetBody.node_id,
          environment: "production",
          execution_mode: targetBody.execution_mode,
          script_path: "",
          parameter_schema: {},
          secret_file_references: [],
          verification_config: {},
          timeout_seconds: targetBody.timeout_seconds,
          privileged_release: true,
          image_spec: targetBody.image_spec,
          status: "active",
          snapshot_hash: "snap-image-1",
          version: 1,
          created_at: "2026-08-02T02:00:00Z",
          updated_at: "2026-08-02T02:00:00Z",
        }, { status: 201 });
      }),
    );
    const user = userEvent.setup();
    await createApp(user, "Redis Test");
    await user.click(screen.getByRole("button", { name: "镜像直连（无需仓库）" }));
    expect(screen.getByRole("heading", { name: "镜像直连部署" })).toBeInTheDocument();
    expect(screen.queryByLabelText("仓库地址")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "继续到部署目标" }));

    await user.click(await screen.findByLabelText("节点"));
    await user.click(await screen.findByRole("option", { name: /生产节点01 · node\.fixture\.invalid/ }));
    expect(screen.getByLabelText("镜像引用")).toHaveValue("docker.io/library/postgres:18-alpine");
    await user.click(await screen.findByRole("checkbox", { name: /compose\.env/ }));
    await user.click(await screen.findByRole("checkbox", { name: /postgres\.env/ }));
    await user.click(screen.getByRole("checkbox", { name: /我确认该镜像、模板与宿主端口/ }));
    expect(screen.getByRole("checkbox", { name: /我确认该镜像、模板与宿主端口/ })).toBeChecked();
    await user.click(screen.getByRole("button", { name: "创建目标" }));

    expect(await screen.findByRole("heading", { name: "应用与镜像部署目标已创建" })).toBeInTheDocument();
    expect(requests).toEqual(["applications.create", "targets.create"]);
    expect(targetBody).toMatchObject({
      node_id: "node-1",
      execution_mode: "image",
      privileged_release: true,
      privileged_release_confirmed: true,
      image_spec: { template: "postgres", image: "docker.io/library/postgres:18-alpine", host_port: 5432, env_files: ["compose.env", "postgres.env"] },
      timeout_seconds: 900,
    });
    expect(targetBody!.secret_file_references).toEqual([]);
  });

  it("来源保存失败时保留已创建应用并提供入口", async () => {
    server.use(
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [], next_cursor: null })),
      http.post("/api/v1/applications", async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "app-wizard", name: body.name, slug: body.slug, description: body.description, status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }, { status: 201 });
      }),
      http.put("/api/v1/applications/app-wizard/source", () => HttpResponse.json({ code: "validation_failed", message: "仓库地址无效", request_id: "req-source-fail" }, { status: 422 })),
    );
    const user = userEvent.setup();
    await createApp(user);
    await fillAndSubmitSource(user);

    expect(await screen.findByRole("alert")).toHaveTextContent("仓库地址无效");
    expect(screen.getByRole("link", { name: "PG Test" })).toHaveAttribute("href", "/apps/app-wizard");
    expect(screen.getByRole("button", { name: "仅创建应用" })).toBeInTheDocument();
  });

  it("目标创建失败时保留应用与来源并允许继续", async () => {
    server.use(
      http.get("/api/v1/git-credentials", () => HttpResponse.json({ items: [credential], next_cursor: null })),
      http.get("/api/v1/agents", () => HttpResponse.json({ items: [agent], next_cursor: null })),
      http.get("/api/v1/nodes", () => HttpResponse.json({ items: [node], next_cursor: null })),
      http.post("/api/v1/applications", async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        return HttpResponse.json({ id: "app-wizard", name: body.name, slug: body.slug, description: body.description, status: "active", version: 1, created_at: "2026-08-02T00:00:00Z", updated_at: "2026-08-02T00:00:00Z" }, { status: 201 });
      }),
      http.put("/api/v1/applications/app-wizard/source", () => HttpResponse.json(draftSource)),
      http.post("/api/v1/applications/app-wizard/source/refreshes", () => HttpResponse.json(queuedDiscovery)),
      http.get("/api/v1/applications/app-wizard/source/refreshes/ref-1", () => HttpResponse.json(succeededDiscovery)),
      http.put("/api/v1/applications/app-wizard/source/branch", () => HttpResponse.json(verifiedSource)),
      http.post("/api/v1/applications/app-wizard/targets", () => HttpResponse.json({ code: "source_not_verified", message: "来源必须已固定", request_id: "req-target-fail" }, { status: 422 })),
    );
    const user = userEvent.setup();
    await createApp(user);
    await fillAndSubmitSource(user);
    await fixMainBranch(user);
    await user.click(screen.getByLabelText("节点"));
    await user.click(await screen.findByRole("option", { name: /生产节点01/ }));
    await user.click(screen.getByRole("button", { name: "创建目标" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("来源必须已固定");
    expect(screen.getByRole("link", { name: "PG Test" })).toHaveAttribute("href", "/apps/app-wizard");
    expect(screen.getByRole("button", { name: "跳过目标" })).toBeInTheDocument();
  });

  it("非管理员直接访问模板创建向导被拒绝", async () => {
    renderRoute("/templates/new", { ...administrator, user: { ...administrator.user!, identity: "user" } });
    expect(await screen.findByRole("heading", { name: "没有访问权限" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "从模板创建应用" })).not.toBeInTheDocument();
  });
});
