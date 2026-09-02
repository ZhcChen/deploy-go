import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const server = setupServer(
  http.get("/api/v1/applications/:applicationId/config-files", () => HttpResponse.json({ items: [], next_cursor: null })),
  http.get("/api/v1/applications/:applicationId/workspace-source", () => HttpResponse.json({ code: "not_found", message: "工作区来源不存在", request_id: "test-workspace-source-missing" }, { status: 404 })),
);
