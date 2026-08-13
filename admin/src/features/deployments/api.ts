import { DeploymentsApi } from "../../api/generated/apis/DeploymentsApi";
import type { DeploymentEventResponse } from "../../api/generated/models/DeploymentEventResponse";
import { apiConfiguration } from "../../api/http-client";

const generatedDeploymentsApi = new DeploymentsApi(apiConfiguration);

export const deploymentsApi = {
  list: (after?: string, limit = 30) => generatedDeploymentsApi.deploymentsList({ limit, after }),
  show: (id: string) => generatedDeploymentsApi.deploymentsShow({ id }),
  events: async (id: string) => {
    const items: DeploymentEventResponse[] = [];
    const cursors = new Set<string>();
    let after: string | undefined;
    do {
      const page = await generatedDeploymentsApi.deploymentsEvents({ id, limit: 200, after });
      items.push(...page.items);
      after = page.nextCursor ?? undefined;
      if (after && cursors.has(after)) throw new Error("部署事件分页返回了重复游标");
      if (after) cursors.add(after);
    } while (after);
    return items;
  },
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
