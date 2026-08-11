import { ExternalKeysApi } from "../../api/generated/apis/ExternalKeysApi";
import { apiConfiguration } from "../../api/http-client";

export const externalKeysApi = new ExternalKeysApi(apiConfiguration);
