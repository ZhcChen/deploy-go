import { SettingsApi } from "../../api/generated/apis/SettingsApi";
import type { RuntimeSettings } from "../../api/generated/models/RuntimeSettings";
import { RuntimeSettingsUpdateToJSON } from "../../api/generated/models/RuntimeSettingsUpdate";
import { apiConfiguration } from "../../api/http-client";

const generatedSettingsApi = new SettingsApi(apiConfiguration);

export const settingsApi = {
  settingsShow: () => generatedSettingsApi.settingsShow(),
  settingsUpdate: (request: { xCSRFToken: string; runtimeSettings: RuntimeSettings }) =>
    generatedSettingsApi.settingsUpdate({ xCSRFToken: request.xCSRFToken, runtimeSettingsUpdate: request.runtimeSettings }, async ({ init }) => ({
      ...init,
      body: RuntimeSettingsUpdateToJSON(request.runtimeSettings) as unknown as BodyInit,
    })),
};
