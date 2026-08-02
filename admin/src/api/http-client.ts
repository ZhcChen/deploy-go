import { AuthApi } from "./generated/apis/AuthApi";
import { ErrorResponseFromJSON } from "./generated/models/ErrorResponse";
import { Configuration, FetchError, ResponseError, type Middleware } from "./generated/runtime";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly requestId?: string,
    readonly details?: unknown,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

type UnauthorizedListener = () => void;
const unauthorizedListeners = new Set<UnauthorizedListener>();

export function onUnauthorized(listener: UnauthorizedListener) {
  unauthorizedListeners.add(listener);
  return () => {
    unauthorizedListeners.delete(listener);
  };
}

const sessionMiddleware: Middleware = {
  async post({ response, url }) {
    if (
      response.status === 401 &&
      !url.endsWith("/api/v1/auth/login") &&
      !url.endsWith("/api/v1/setup")
    ) {
      unauthorizedListeners.forEach((listener) => listener());
    }
  },
};

const basePath = import.meta.env.VITE_API_BASE_URL?.replace(/\/+$/, "") ?? "";
export const apiConfiguration = new Configuration({
  basePath,
  credentials: "include",
  middleware: [sessionMiddleware],
});
export const authApi = new AuthApi(apiConfiguration);

export async function normalizeApiError(error: unknown): Promise<ApiError> {
  if (error instanceof ApiError) return error;
  if (error instanceof ResponseError) {
    const requestId = error.response.headers.get("X-Request-ID") ?? undefined;
    try {
      const body = ErrorResponseFromJSON(await error.response.clone().json());
      return new ApiError(
        error.response.status,
        body.code,
        body.message,
        body.requestId || requestId,
        body.details,
      );
    } catch {
      return new ApiError(
        error.response.status,
        "unexpected_response",
        "服务返回了无法识别的错误",
        requestId,
      );
    }
  }
  if (error instanceof FetchError) {
    return new ApiError(0, "network_error", "无法连接部署控制服务");
  }
  return new ApiError(0, "unexpected_error", "请求未能完成");
}
