import { ApplicationConfigsApi } from "../../api/generated/apis/ApplicationConfigsApi";
import { apiConfiguration } from "../../api/http-client";

export const applicationConfigsApi = new ApplicationConfigsApi(apiConfiguration);
