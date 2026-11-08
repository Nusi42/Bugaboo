import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useChangeAi, type AiSetup } from "./ai.ts";
import { SwitchBox } from "./ControlWithHint.tsx";

/**
 * The two switches of the setup: whether AI is on for the organization (off at the start, AI-02), and
 * whether names of people go to an outside provider, otherwise their role (PRIV-08, M7-Q5). An outside
 * provider cannot be switched on before the agreement is confirmed; the switch waits and says so.
 */
export function AiSwitches({ setup }: { readonly setup: AiSetup }) {
  const change = useChangeAi();
  const waitsForAgreement = setup.agreement_status === "needed" && !setup.enabled;
  const outside = setup.agreement_status !== "not-needed";
  return (
    <div {...partOf(part.stack)}>
      <SwitchBox
        label={t("setting.ai.enabled.name")}
        isSelected={setup.enabled}
        onChange={(enabled) => {
          change.mutate({ body: { enabled } });
        }}
        help={t("setting.ai.enabled.help")}
      >
        {waitsForAgreement ? (
          <Alert tone={alertTone.warning}>{t("ai-setup.switch-needs-agreement")}</Alert>
        ) : null}
      </SwitchBox>
      {outside ? (
        <SwitchBox
          label={t("setting.ai.send-names.name")}
          isSelected={setup.send_names}
          onChange={(sendNames) => {
            change.mutate({ body: { send_names: sendNames } });
          }}
          help={t("setting.ai.send-names.help")}
        />
      ) : null}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}
