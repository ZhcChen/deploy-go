import { SshCredentialsApi } from "../../api/generated/apis/SshCredentialsApi";
import { NodesApi } from "../../api/generated/apis/NodesApi";
import { apiConfiguration } from "../../api/http-client";

export const sshCredentialsApi = new SshCredentialsApi(apiConfiguration);
export const nodesApi = new NodesApi(apiConfiguration);
