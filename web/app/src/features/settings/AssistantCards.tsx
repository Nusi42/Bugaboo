import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { type AiSetup } from "./ai.ts";
import { AiAgreement } from "./AiAgreement.tsx";
import { AiSetupForm } from "./AiSetupForm.tsx";
import { AiSwitches } from "./AiSwitches.tsx";
import { AiWhatGoesWhere } from "./AiWhatGoesWhere.tsx";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings of the assistant itself that the one settings form draws: its look-ups. */
const ASSISTANT_TOPICS = "assistant.";

/**
 * Everything that is only about the assistant (WP-15), after the AI apps on the page: how much it may
 * look up for one question, then the setup from the provider to the switch in the order an
 * administrator goes through it: the provider, the agreement for an outside provider, what goes
 * where, the switch.
 */
export function AssistantCards({ setup }: { readonly setup: AiSetup }) {
  return (
    <>
      <Card heading={t("ai-setup.assistant")} intro={t("ai-setup.assistant-intro")}>
        <OrganizationTopic topics={ASSISTANT_TOPICS} searchable={false} />
      </Card>
      <Card heading={t("ai-setup.set-up")} intro={t("ai-setup.set-up-intro")}>
        <AiSetupForm setup={setup} />
      </Card>
      <Card heading={t("ai-setup.agreement")} intro={t("ai-setup.agreement-intro")}>
        <AiAgreement setup={setup} />
      </Card>
      <Card heading={t("ai-setup.where")} intro={t("ai-setup.where-intro")}>
        <AiWhatGoesWhere />
      </Card>
      <Card heading={t("ai-setup.switch")}>
        <AiSwitches setup={setup} />
      </Card>
    </>
  );
}
