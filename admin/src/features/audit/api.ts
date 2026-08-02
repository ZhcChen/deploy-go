import { AuditApi } from "../../api/generated/apis/AuditApi";
import { apiConfiguration } from "../../api/http-client";

export const auditApi = new AuditApi(apiConfiguration);
