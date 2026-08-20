import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const server = setupServer(
  http.get("/api/v1/applications/:applicationId/config-files", () => HttpResponse.json({ items: [], next_cursor: null })),
);
