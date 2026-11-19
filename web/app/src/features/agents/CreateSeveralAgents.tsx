import { agentsMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { CreateAgentButton } from "./CreateAgentDialog.tsx";
import { SeveralAgentsForm } from "./SeveralAgentsForm.tsx";
import { SeveralKeysShown } from "./SeveralKeysShown.tsx";
import type { CreatedAgent } from "./several-agents.ts";

/** What *Create several agents* shows: the button, the form, or the keys that were just made. */
type Step =
  | { readonly step: "idle" }
  | { readonly step: "form" }
  | { readonly step: "shown"; readonly created: readonly CreatedAgent[] };

/**
 * The buttons under the list of agents: *Create an agent*, and *Create several agents* (ADR-0029,
 * amendment M9), which opens its form below them and then shows the keys it made, once.
 */
export function CreateAgentsRow({ count }: { readonly count: number }) {
  const [step, setStep] = useState<Step>({ step: "idle" });
  const idle = (): void => {
    setStep({ step: "idle" });
  };
  return (
    <>
      <div {...partOf(part.row)}>
        <CreateAgentButton count={count} />
        {step.step === "idle" ? (
          <Button
            isDisabled={count >= agentsMost}
            onPress={() => {
              setStep({ step: "form" });
            }}
          >
            {t("agents.several")}
          </Button>
        ) : null}
      </div>
      {step.step === "form" ? (
        <Card heading={t("agents.several")} intro={t("agents.several-text")}>
          <SeveralAgentsForm
            onCreated={(created) => {
              setStep({ step: "shown", created });
            }}
            onCancel={idle}
          />
        </Card>
      ) : null}
      {step.step === "shown" ? <SeveralKeysShown created={step.created} onDone={idle} /> : null}
    </>
  );
}
