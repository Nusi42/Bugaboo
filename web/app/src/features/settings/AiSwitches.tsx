import { t } from "@bugaboo/messages";
import { Alert, alertTone, Switch, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useChangeAi, type AiSetup } from "./ai.ts";

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
      <div {...partOf(part.column)}>
        <Switch
          label={t("setting.ai.enabled.name")}
          isSelected={setup.enabled}
          onChange={(enabled) => {
            change.mutate({ body: { enabled } });
          }}
        />
        <p {...partOf(part.muted)}>{t("setting.ai.enabled.help")}</p>
        {waitsForAgreement ? (
          <Alert tone={alertTone.warning}>{t("ai-setup.switch-needs-agreement")}</Alert>
        ) : null}
      </div>
      {outside ? (
        <div {...partOf(part.column)}>
          <Switch
            label={t("setting.ai.send-names.name")}
            isSelected={setup.send_names}
            onChange={(sendNames) => {
              change.mutate({ body: { send_names: sendNames } });
            }}
          />
          <p {...partOf(part.muted)}>{t("setting.ai.send-names.help")}</p>
        </div>
      ) : null}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}
