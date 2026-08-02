import { SshCredentialsApi } from "../../api/generated/apis/SshCredentialsApi";
import { NodesApi } from "../../api/generated/apis/NodesApi";
import { apiConfiguration } from "../../api/http-client";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";

export const sshCredentialsApi = new SshCredentialsApi(apiConfiguration);
export const nodesApi = new NodesApi(apiConfiguration);

export async function listAllNodes() {
  const items: NodeResponse[] = [];
  let after: string | undefined;
  do {
    const page = await nodesApi.nodesList({ limit: 200, after });
    items.push(...page.items);
    after = page.nextCursor ?? undefined;
  } while (after);
  return items;
}
