import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { SettingsForm } from "../settings/index.ts";
import { usePersonalForm } from "./use-personal-form.ts";

/**
 * *My settings › Assistant* (AI-10, WP-15): how many look-ups the assistant may use for one of your questions. Your
 * number can be lower than the most your administrators set, never higher; the page is only about the assistant.
 */
export function AssistantPage() {
  usePageTitle(t("nav.assistant-settings"));
  const form = usePersonalForm(["assistant."]);
  return (
    <Card heading={t("nav.assistant-settings")} level={1} intro={t("setting.page.assistant-intro")}>
      <SettingsForm {...form} searchable={false} />
    </Card>
  );
}
