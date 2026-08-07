import { DeploymentsApi } from "../../api/generated/apis/DeploymentsApi";
import { apiConfiguration } from "../../api/http-client";

const generatedDeploymentsApi = new DeploymentsApi(apiConfiguration);

export const deploymentsApi = {
  list: (after?: string) => generatedDeploymentsApi.deploymentsList({ limit: 30, after }),
  show: (id: string) => generatedDeploymentsApi.deploymentsShow({ id }),
  preview: (id: string, csrfToken: string, parameters: unknown, releaseStrategy: "automatic" | "manual") => generatedDeploymentsApi.applicationDeploymentsPreview({ id, xCSRFToken: csrfToken, previewRequest: { parameters, releaseStrategy } }),
  confirm: (id: string, csrfToken: string, idempotencyKey: string, snapshotHash: string, parameters: unknown, releaseStrategy: "automatic" | "manual", releaseVersion?: string) => generatedDeploymentsApi.applicationDeploymentsConfirm(
    { id, xCSRFToken: csrfToken, confirmRequest: { snapshotHash, parameters, releaseStrategy, releaseVersion } },
    async ({ init }) => ({ ...init, headers: { ...init.headers, "Idempotency-Key": idempotencyKey } }),
  ),
  cancel: (id: string, csrfToken: string) => generatedDeploymentsApi.deploymentsCancel({ id, xCSRFToken: csrfToken }),
  retry: (id: string, csrfToken: string, idempotencyKey: string) => generatedDeploymentsApi.deploymentsRetry(
    { id, xCSRFToken: csrfToken },
    async ({ init }) => ({ ...init, headers: { ...init.headers, "Idempotency-Key": idempotencyKey } }),
  ),
  release: (id: string, csrfToken: string) => generatedDeploymentsApi.deploymentsRelease({ id, xCSRFToken: csrfToken }),
};

export function createIdempotencyKey(prefix: "deploy" | "retry") {
  return `${prefix}-${crypto.randomUUID()}`;
}
