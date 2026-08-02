import type { ErrorResponse } from "./generated/models/ErrorResponse";

export interface ApiRequestContext {
  credentials: "include";
  csrfToken?: string;
}

export interface CursorPage<T> {
  items: T[];
  nextCursor?: string | null;
}

export interface ApiFailure {
  status: number;
  error: ErrorResponse;
}

export interface SseResumeCursor {
  lastEventId?: string;
  after?: number;
}

export interface SseMessage {
  id: string;
  event: string;
  data: string;
}
