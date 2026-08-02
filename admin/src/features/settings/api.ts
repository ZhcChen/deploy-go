import { SettingsApi } from "../../api/generated/apis/SettingsApi";
import type { RuntimeSettings } from "../../api/generated/models/RuntimeSettings";
import { RuntimeSettingsToJSON } from "../../api/generated/models/RuntimeSettings";
import { apiConfiguration } from "../../api/http-client";

const generatedSettingsApi = new SettingsApi(apiConfiguration);

export const settingsApi = {
  settingsShow: () => generatedSettingsApi.settingsShow(),
  settingsUpdate: (request: { xCSRFToken: string; runtimeSettings: RuntimeSettings }) =>
    generatedSettingsApi.settingsUpdate(request, async ({ init }) => ({
      ...init,
      body: RuntimeSettingsToJSON(request.runtimeSettings) as unknown as BodyInit,
    })),
};
