import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import type { Setting } from "../../shared/my-settings.ts";
import { AiAppAddresses } from "./AiAppAddresses.tsx";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings of the apps that may sign in, which the one settings form draws. */
const APP_TOPICS = "ai-apps.";

/** The explanation of *Let AI apps sign in*: one sentence, for the state the switch is in. */
const APP_HELPS = {
  "ai-apps.enabled": (setting: Setting): string =>
    setting.value === true ? t("ai-setup.apps-on") : t("ai-setup.apps-off"),
};

/**
 * *Apps that may sign in* (AI-22, M7-Q4, M7-F42): whether AI apps can sign in at all, which the one
 * settings form draws like any other setting, and the web addresses an administrator allows.
 */
export function AiAppsCard() {
  return (
    <Card heading={t("ai-setup.apps")} intro={t("ai-setup.apps-intro")}>
      <OrganizationTopic topics={APP_TOPICS} searchable={false} helps={APP_HELPS} />
      <AiAppAddresses />
    </Card>
  );
}
