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

export type EnvEditorMode = "structured" | "raw";

export interface DotenvError {
  line: number;
  code: string;
  message: string;
}

export interface DotenvAssignment {
  kind: "assignment";
  key: string;
  value: string;
  quote: "'" | '"' | null;
}

export type DotenvLine = DotenvAssignment | { kind: "blank" | "comment" | "invalid"; raw: string };

export interface DotenvDocument {
  lines: DotenvLine[];
  errors: DotenvError[];
}
