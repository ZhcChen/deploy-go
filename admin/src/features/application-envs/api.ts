import { ApplicationEnvsApi } from "../../api/generated/apis/ApplicationEnvsApi";
import { apiConfiguration } from "../../api/http-client";

export const applicationEnvsApi = new ApplicationEnvsApi(apiConfiguration);
