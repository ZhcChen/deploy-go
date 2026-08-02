import type { DeploymentLogResponse } from "../../api/generated/models/DeploymentLogResponse";

export const LOG_WINDOW_SIZE = 1000;

export function appendDeploymentLog(logs: DeploymentLogResponse[], incoming: DeploymentLogResponse, limit = LOG_WINDOW_SIZE) {
  if (logs.some((log) => log.sequence === incoming.sequence)) return logs;
  const next = [...logs, { ...incoming, content: sanitizeLogText(incoming.content) }]
    .sort((left, right) => left.sequence - right.sequence);
  return next.length > limit ? next.slice(next.length - limit) : next;
}

export function sanitizeLogText(value: string) {
  return Array.from(value, (character) => {
    const code = character.codePointAt(0) ?? 0;
    const unsafeControl = code <= 8 || code === 11 || code === 12 || (code >= 14 && code <= 31) || (code >= 127 && code <= 159);
    const unsafeDirection = (code >= 0x202a && code <= 0x202e) || (code >= 0x2066 && code <= 0x2069);
    return unsafeControl || unsafeDirection ? "�" : character;
  }).join("");
}
