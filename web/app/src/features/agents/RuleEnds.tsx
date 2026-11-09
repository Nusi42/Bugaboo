import { agentRuleDaysDefault, agentRuleDaysMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Checkbox, TextField, partOf, part } from "@bugaboo/ui";

import { useAgentNamed, useMyAgentWork } from "./agents.ts";
import { daysAreRight, type DraftProps } from "./rule-draft.ts";

/**
 * What a rule with no end means, in the caution color: it goes on handing work to the agent until
 * the person stops it, also when nobody looks; and more strongly when it hands over and accepts
 * results by itself.
 */
function NeverWarning({ draft }: Pick<DraftProps, "draft">) {
  const agent = useAgentNamed(draft.agent)?.name ?? t("agents.rule-its-agent");
  const alone = draft.way === "hand-over" && draft.handOver.acceptByItself;
  return (
    <Alert tone={alertTone.warning}>
      {t(alone ? "agents.rule-never-ends-alone" : "agents.rule-never-ends-help", { agent })}
    </Alert>
  );
}

/** When the rule runs out: the days, or *Never runs out* where an administrator allows it. */
export function RuleEnds({ draft, onChange }: DraftProps) {
  const allowed = useMyAgentWork().data?.rules_allowed.never_run_out === true;
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
        <NeverWarning draft={draft} />
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
