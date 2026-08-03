import { NodesApi } from "../../api/generated/apis/NodesApi";
import { apiConfiguration } from "../../api/http-client";

export const nodesApi = new NodesApi(apiConfiguration);
