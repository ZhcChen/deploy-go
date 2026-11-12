import { focusManager, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { http, HttpResponse } from "msw";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AppProviders } from "../app/AppProviders";
import type { DeploymentResponse } from "../api/generated/models/DeploymentResponse";
import type { AuthSnapshot } from "../features/auth/AuthContext";
import { DeploymentFlowPanel } from "../features/deployments/DeploymentFlowPanel";
import { DeploymentLogPanel } from "../features/deployments/DeploymentLogPanel";
import { AppRoutes } from "../routes/AppRoutes";
import { server } from "./server";

const administrator: AuthSnapshot = { status: "authenticated", csrfToken: "csrf-live", user: { id: "admin-1", username: "admin", displayName: "管理员", identity: "administrator" } };
const deploymentJson = { id: "deployment-live", application_id: "app-1", target_id: "target-1", target_runs: [], requested_by: "admin-1", status: "running", phase: "deploying", execution_mode: "script", stage_tasks: [], snapshot_hash: "snapshot-live", protocol_complete: false, queued_at: "2026-11-12T00:00:00Z", started_at: "2026-11-12T00:00:01Z", created_at: "2026-11-12T00:00:00Z", updated_at: "2026-11-12T00:00:01Z", version: 1 };
const startedEvent = { id: "event-1", stage: "release", event_name: "deploy.step.started", status: "running", step_id: "restart", step: "重启服务", created_at: "2026-11-12T00:00:02Z" };
const succeededEvent = { ...startedEvent, id: "event-2", event_name: "deploy.step.succeeded", status: "succeeded", created_at: "2026-11-12T00:00:09Z" };

function flowDeployment(status: string) {
  return { id: "deployment-live", status, phase: status, protocolComplete: true, stageTasks: [], targetRuns: [] } as unknown as DeploymentResponse;
}

function renderPanel(deployment: DeploymentResponse) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const tree = (current: DeploymentResponse) => <QueryClientProvider client={client}><DeploymentFlowPanel deployment={current} onViewLogs={() => undefined} /></QueryClientProvider>;
  const view = render(tree(deployment));
  return { update: (next: DeploymentResponse) => view.rerender(tree(next)) };
}

function releaseStageText() {
  return within(screen.getByRole("button", { name: /发布 release/ })).getByText(/^(未执行|执行中|成功|失败|已取消|执行中断)$/).textContent;
}

function renderRoute(path: string) {
  const router = createMemoryRouter([{ path: "*", element: <AppRoutes /> }], { initialEntries: [path] });
  return render(<AppProviders initialAuth={administrator}><RouterProvider router={router} /></AppProviders>);
}

describe("部署详情页的自动刷新", () => {
  afterEach(() => focusManager.setFocused(undefined));

  describe("流程图在部署结束时与结果一致", () => {
    it.each(["succeeded", "failed", "canceled", "interrupted"])("部署变成 %s 后补刷一次事件，流程图显示最终状态", async (finalStatus) => {
      let eventCalls = 0;
      server.use(http.get("/api/v1/deployments/deployment-live/events", () => {
        eventCalls += 1;
        // 最后一条事件在上一次轮询之后才落库：第一次只有“开始”，之后才有“成功”。
        return HttpResponse.json({ items: eventCalls === 1 ? [startedEvent] : [startedEvent, succeededEvent], next_cursor: null });
      }));
      const panel = renderPanel(flowDeployment("running"));
      await waitFor(() => expect(releaseStageText()).toBe("执行中"));
      expect(eventCalls).toBe(1);

      panel.update(flowDeployment(finalStatus));

      await waitFor(() => expect(eventCalls).toBe(2));
      // 事件优先于阶段状态：补刷之后“成功”事件到位，阶段不再停在“执行中”。
      await waitFor(() => expect(releaseStageText()).toBe("成功"));
    });

    it("首次加载就是终态的部署不重复请求事件", async () => {
      let eventCalls = 0;
      server.use(http.get("/api/v1/deployments/deployment-live/events", () => {
        eventCalls += 1;
        return HttpResponse.json({ items: [startedEvent, succeededEvent], next_cursor: null });
      }));
      renderPanel(flowDeployment("succeeded"));
      await waitFor(() => expect(releaseStageText()).toBe("成功"));
      await new Promise((resolve) => setTimeout(resolve, 150));
      expect(eventCalls).toBe(1);
    });
  });

  describe("切回标签页立即刷新", () => {
    it("运行中的部署：详情和流程事件都立即重新请求", async () => {
      let detailCalls = 0;
      let eventCalls = 0;
      server.use(
        http.get("/api/v1/deployments/deployment-live", () => { detailCalls += 1; return HttpResponse.json(deploymentJson); }),
        http.get("/api/v1/deployments/deployment-live/events", () => { eventCalls += 1; return HttpResponse.json({ items: [], next_cursor: null }); }),
      );
      renderRoute("/deployments/deployment-live");
      await screen.findAllByText("运行中");
      await waitFor(() => expect(eventCalls).toBe(1));
      expect(detailCalls).toBe(1);

      act(() => { focusManager.setFocused(false); focusManager.setFocused(true); });

      await waitFor(() => expect(detailCalls).toBe(2));
      await waitFor(() => expect(eventCalls).toBe(2));
    });

    it("已终结的部署切回标签页不发请求", async () => {
      let detailCalls = 0;
      let eventCalls = 0;
      server.use(
        http.get("/api/v1/deployments/deployment-live", () => { detailCalls += 1; return HttpResponse.json({ ...deploymentJson, status: "succeeded", phase: "succeeded", protocol_complete: true, finished_at: "2026-11-12T00:00:10Z" }); }),
        http.get("/api/v1/deployments/deployment-live/events", () => { eventCalls += 1; return HttpResponse.json({ items: [], next_cursor: null }); }),
      );
      renderRoute("/deployments/deployment-live");
      await screen.findAllByText("成功");
      await waitFor(() => expect(eventCalls).toBe(1));

      act(() => { focusManager.setFocused(false); focusManager.setFocused(true); });
      await new Promise((resolve) => setTimeout(resolve, 150));

      expect(detailCalls).toBe(1);
      expect(eventCalls).toBe(1);
    });
  });

  describe("日志流断开后自动恢复", () => {
    it("放弃重连、停在“连接已断开”之后，联网时自动重连并收到终态", async () => {
      let respondWithTerminal = false;
      const onTerminal = vi.fn();
      server.use(http.get("/api/v1/deployments/deployment-live/logs", () => respondWithTerminal
        ? new HttpResponse('event: terminal\ndata: {"status":"succeeded","last_event_id":0}\n\n', { headers: { "Content-Type": "text/event-stream" } })
        // 401 不重试，立刻进入“连接已断开”，不依赖退避计时器。
        : HttpResponse.json({ code: "unauthorized", message: "需要登录", request_id: "req_live" }, { status: 401 })));
      render(<DeploymentLogPanel deploymentId="deployment-live" onTerminal={onTerminal} onAuthorizationRevoked={() => undefined} />);
      expect(await screen.findByText("连接已断开")).toBeInTheDocument();

      respondWithTerminal = true;
      act(() => { window.dispatchEvent(new Event("online")); });

      expect(await screen.findByText("已结束")).toBeInTheDocument();
      expect(onTerminal).toHaveBeenCalledTimes(1);
    });
  });
});
