import { DeploymentsApi } from "../../api/generated/apis/DeploymentsApi";
import { apiConfiguration } from "../../api/http-client";

const generatedDeploymentsApi = new DeploymentsApi(apiConfiguration);

export const deploymentsApi = {
  list: (after?: string) => generatedDeploymentsApi.deploymentsList({ limit: 30, after }),
  show: (id: string) => generatedDeploymentsApi.deploymentsShow({ id }),
  preview: (id: string, csrfToken: string, parameters: unknown) => generatedDeploymentsApi.applicationDeploymentsPreview({ id, xCSRFToken: csrfToken, previewRequest: { parameters } }),
  confirm: (id: string, csrfToken: string, idempotencyKey: string, snapshotHash: string, parameters: unknown) => generatedDeploymentsApi.applicationDeploymentsConfirm(
    { id, xCSRFToken: csrfToken, confirmRequest: { snapshotHash, parameters } },
    async ({ init }) => ({ ...init, headers: { ...init.headers, "Idempotency-Key": idempotencyKey } }),
  ),
  cancel: (id: string, csrfToken: string) => generatedDeploymentsApi.deploymentsCancel({ id, xCSRFToken: csrfToken }),
  retry: (id: string, csrfToken: string, idempotencyKey: string) => generatedDeploymentsApi.deploymentsRetry(
    { id, xCSRFToken: csrfToken },
    async ({ init }) => ({ ...init, headers: { ...init.headers, "Idempotency-Key": idempotencyKey } }),
  ),
};

export function createIdempotencyKey(prefix: "deploy" | "retry") {
  return `${prefix}-${crypto.randomUUID()}`;
}
