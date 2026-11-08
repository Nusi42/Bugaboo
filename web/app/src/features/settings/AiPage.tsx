import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { useAiSetup } from "./ai.ts";
import { AiAgreement } from "./AiAgreement.tsx";
import { AiAppsCard } from "./AiAppsCard.tsx";
import { AiSetupForm } from "./AiSetupForm.tsx";
import { AiSwitches } from "./AiSwitches.tsx";
import { AiWhatGoesWhere } from "./AiWhatGoesWhere.tsx";

/**
 * *Organization settings › Connections › AI* (AI-01, AI-02, PRIV-08, COMP-08, AI-22): *Set up the
 * assistant* from the provider to the switch, in the order an administrator goes through it: the
 * provider, the agreement for an outside provider, what goes where, the switch, and the AI apps
 * that may sign in. AI stays off until it is switched on here.
 */
export function AiPage() {
  usePageTitle(t("ai-setup.title"));
  const setup = useAiSetup();
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("ai-setup.title")} level={1} intro={t("ai-setup.intro")} />
      <QueryGate error={setup.error} data={setup.data} loading={null}>
        {(shown) => (
          <>
            <Card heading={t("ai-setup.set-up")} intro={t("ai-setup.set-up-intro")}>
              <AiSetupForm setup={shown} />
            </Card>
            <Card heading={t("ai-setup.agreement")} intro={t("ai-setup.agreement-intro")}>
              <AiAgreement setup={shown} />
            </Card>
            <Card heading={t("ai-setup.where")} intro={t("ai-setup.where-intro")}>
              <AiWhatGoesWhere />
            </Card>
            <Card heading={t("ai-setup.switch")}>
              <AiSwitches setup={shown} />
            </Card>
          </>
        )}
      </QueryGate>
      <AiAppsCard />
    </div>
  );
}
