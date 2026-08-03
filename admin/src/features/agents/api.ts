import { AgentsApi } from "../../api/generated/apis/AgentsApi";
import { apiConfiguration } from "../../api/http-client";

export const agentsApi = new AgentsApi(apiConfiguration);
