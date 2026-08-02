import { ApplicationsApi } from "../../api/generated/apis/ApplicationsApi";
import { DeploymentTargetsApi } from "../../api/generated/apis/DeploymentTargetsApi";
import { GrantsApi } from "../../api/generated/apis/GrantsApi";
import { NodesApi } from "../../api/generated/apis/NodesApi";
import { UsersApi } from "../../api/generated/apis/UsersApi";
import { apiConfiguration } from "../../api/http-client";

export const applicationsApi = new ApplicationsApi(apiConfiguration);
export const deploymentTargetsApi = new DeploymentTargetsApi(apiConfiguration);
export const grantsApi = new GrantsApi(apiConfiguration);
export const applicationNodesApi = new NodesApi(apiConfiguration);
export const grantUsersApi = new UsersApi(apiConfiguration);
