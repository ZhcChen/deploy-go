import { ApiError } from "../../api/http-client";

export function toNotice(error: unknown): ApiError {
  return error instanceof Error && "status" in error
    ? (error as ApiError)
    : new ApiError(
        0,
        "unexpected_error",
        error instanceof Error ? error.message : "请求未能完成",
      );
}
