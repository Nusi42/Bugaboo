import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatDate } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { agentStateText } from "../../shared/agent-state.ts";
import type { Work } from "./agent-work.ts";
import { outcomeOf, outcomeText } from "./outcome.ts";

/**
 * Who works and where the work is: *Fixer, agent of Ada Brandt* (as *Who sees people* shows her),
 * who handed the issue over and when, and the state in words with its mark (UX §4.17).
 */
export function WorkHead({ work }: { readonly work: Work }) {
  // Once the work is over by a review the pill says how it ended, not where it stood.
  const outcome = outcomeOf(work);
  return (
    <div {...partOf(part.agentWorkHead)}>
      <div {...partOf(part.agentWorkWho)}>
        <span>
          {t("agent-work.agent-of", {
            agent: work.agent_name,
            person: personName(work.agent_person),
          })}
        </span>
        <span {...partOf(part.muted)}>
          {t("agent-work.handed-over", {
            person: personName(work.handed_over_by),
            date: formatDate(work.handed_over_at),
          })}
        </span>
      </div>
      <span {...partOf(part.agentWorkPill, { variants: [outcome ?? work.state] })}>
        {outcome === undefined ? agentStateText(work.state) : outcomeText(outcome)}
      </span>
    </div>
  );
}
