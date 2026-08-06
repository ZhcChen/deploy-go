import { GitCredentialsApi } from "../../api/generated/apis/GitCredentialsApi";
import { apiConfiguration } from "../../api/http-client";

export const gitCredentialsApi = new GitCredentialsApi(apiConfiguration);
