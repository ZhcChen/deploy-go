import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const server = setupServer(
  http.get("/api/v1/applications/:applicationId/runtime-status", () =>
    HttpResponse.json(
      { code: "not_found", message: "尚未读取该目标的运行时状态", request_id: "runtime-status-empty" },
      { status: 404 },
    ),
  ),
);
