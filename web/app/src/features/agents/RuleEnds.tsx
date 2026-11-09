import { agentRuleDaysDefault, agentRuleDaysMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Checkbox, TextField, partOf, part } from "@bugaboo/ui";

import { useMyAgentWork } from "./agents.ts";
import { daysAreRight, type DraftProps } from "./rule-draft.ts";

/** When the rule runs out: the days, or *Never runs out* where an administrator allows it. */
export function RuleEnds({ draft, onChange }: DraftProps) {
  const allowed = useMyAgentWork().data?.rules_never_run_out === true;
  return (
    <div {...partOf(part.stack)}>
      {allowed ? (
        <Checkbox
          label={t("agents.rule-never-ends")}
          isSelected={draft.neverEnds}
          onChange={(neverEnds) => {
            onChange({ neverEnds });
          }}
        />
      ) : null}
      {allowed && draft.neverEnds ? (
        <p {...partOf(part.muted)}>{t("agents.rule-never-ends-help")}</p>
      ) : (
        <TextField
          label={t("agents.rule-days")}
          description={t("agents.rule-days-help", {
            usual: agentRuleDaysDefault,
            most: agentRuleDaysMost,
          })}
          errorMessage={
            daysAreRight(draft.days)
              ? undefined
              : t("agents.rule-days-wrong", { most: agentRuleDaysMost })
          }
          value={draft.days}
          onChange={(days) => {
            onChange({ days });
          }}
        />
      )}
    </div>
  );
}
