import { NodesApi } from "../../api/generated/apis/NodesApi";
import { apiConfiguration, apiFetch } from "../../api/http-client";

export const nodesApi = new NodesApi(apiConfiguration);

export type TerminalCapability = {
  nodeId: string;
  privilegedExecution: boolean;
  available: boolean;
  unavailableCode: string | null;
  agentId: string | null;
  agentOnline: boolean;
  identityValid: boolean;
  protocolVersion: number | null;
  ptyTerminal: boolean;
};

export type TerminalSession = {
  id: string;
  nodeId: string;
  agentId: string;
  status: string;
};

type CapabilityJson = {
  node_id: string;
  privileged_execution: boolean;
  available: boolean;
  unavailable_code?: string | null;
  agent_id?: string | null;
  agent_online: boolean;
  identity_valid: boolean;
  protocol_version?: number | null;
  pty_terminal: boolean;
};

type SessionJson = {
  id: string;
  node_id: string;
  agent_id: string;
  status: string;
};

export const terminalApi = {
  async capability(nodeId: string): Promise<TerminalCapability> {
    const response = await apiFetch(`/api/v1/nodes/${encodeURIComponent(nodeId)}/terminal-capability`);
    const value = await response.json() as CapabilityJson;
    return {
      nodeId: value.node_id,
      privilegedExecution: value.privileged_execution,
      available: value.available,
      unavailableCode: value.unavailable_code ?? null,
      agentId: value.agent_id ?? null,
      agentOnline: value.agent_online,
      identityValid: value.identity_valid,
      protocolVersion: value.protocol_version ?? null,
      ptyTerminal: value.pty_terminal,
    };
  },
  async updatePrivilegedExecution(nodeId: string, enabled: boolean, csrfToken: string) {
    await apiFetch(`/api/v1/nodes/${encodeURIComponent(nodeId)}/privileged-execution`, {
      method: "PUT",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": csrfToken },
      body: JSON.stringify({ enabled }),
    });
  },
  async createSession(nodeId: string, csrfToken: string): Promise<TerminalSession> {
    const response = await apiFetch(`/api/v1/nodes/${encodeURIComponent(nodeId)}/terminal-sessions`, {
      method: "POST",
      headers: { "X-CSRF-Token": csrfToken },
    });
    const value = await response.json() as SessionJson;
    return { id: value.id, nodeId: value.node_id, agentId: value.agent_id, status: value.status };
  },
  async closeSession(sessionId: string, csrfToken: string) {
    await apiFetch(`/api/v1/terminal-sessions/${encodeURIComponent(sessionId)}/close`, {
      method: "POST",
      headers: { "X-CSRF-Token": csrfToken },
    });
  },
};
