import { t } from "@bugaboo/messages";
import { Checkbox, partOf, part, TextField } from "@bugaboo/ui";

import type { DraftProps } from "./hand-over-draft.ts";

/** The three numbers of the budget, side by side where there is room. */
function BudgetNumbers({ draft, onChange }: DraftProps) {
  return (
    <div {...partOf(part.agentWorkNumbers)}>
      <TextField
        label={t("agent-work.budget-attempts")}
        value={draft.attempts}
        onChange={(attempts) => {
          onChange({ attempts });
        }}
      />
      <TextField
        label={t("agent-work.budget-minutes")}
        value={draft.minutes}
        onChange={(minutes) => {
          onChange({ minutes });
        }}
      />
      <TextField
        label={t("agent-work.budget-cost")}
        value={draft.cost}
        onChange={(cost) => {
          onChange({ cost });
        }}
      />
    </div>
  );
}

/**
 * The budget and the end of a hand-over, and whether each plan waits for approval (UX §4.17). The
 * numbers are the agent's own limits and what it reports; Bugaboo stops nothing by them but the
 * attempts. The end left empty is the usual number of days of the organization.
 */
export function HandOverLimits({ draft, onChange }: DraftProps) {
  return (
    <fieldset {...partOf(part.agentWorkGroup)}>
      <legend {...partOf(part.agentWorkGroupName)}>{t("agent-work.budget")}</legend>
      <BudgetNumbers draft={draft} onChange={onChange} />
      <Checkbox
        label={t("agent-work.plan-needs-approval")}
        isSelected={draft.planNeedsApproval}
        onChange={(on) => {
          onChange({ planNeedsApproval: on });
        }}
      />
      <Checkbox
        label={t("agent-work.accept-by-itself")}
        isSelected={draft.acceptByItself}
        onChange={(on) => {
          onChange({ acceptByItself: on });
        }}
      />
      <p {...partOf(part.muted)}>{t("agent-work.accept-by-itself-help")}</p>
      <TextField
        label={t("agent-work.days")}
        description={t("agent-work.days-help")}
        value={draft.days}
        onChange={(days) => {
          onChange({ days });
        }}
      />
    </fieldset>
  );
}
