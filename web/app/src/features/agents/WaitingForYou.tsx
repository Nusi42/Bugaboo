import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useNow } from "../../shared/use-now.ts";
import { runsOutSoon, type ListedAgent, type MyWorkItem, type Rule } from "./agents.ts";
import { waitingInOrder } from "./agent-state.ts";
import { type RuleNames } from "./rule-names.ts";
import { Section } from "./Section.tsx";
import { WorkRow } from "./WaitingWork.tsx";
import { RuleRow } from "./WaitingRule.tsx";

export interface WaitingForYouProps {
  readonly work: readonly MyWorkItem[];
  readonly rules: readonly Rule[];
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
}

/**
 * *Waiting for you* (UX §4.17): the questions, the plans to approve, the results to review and the
 * standing rules about to run out, each with a button to the place where it is answered.
 */
export function WaitingForYou({ work, rules, agents, names }: WaitingForYouProps) {
  const now = useNow();
  const waiting = waitingInOrder(work);
  const ending = rules.filter((rule) => runsOutSoon(rule, now));
  const count = waiting.length + ending.length;
  return (
    <Section heading={t("agent-work.state.waiting-for-you")} count={count}>
      {count === 0 ? (
        <p {...partOf(part.muted)}>{t("agents.waiting-none")}</p>
      ) : (
        <ul {...partOf(part.list)}>
          {waiting.map((item) => (
            <WorkRow key={item.work.id} item={item} agents={agents} />
          ))}
          {ending.map((rule) => (
            <RuleRow key={rule.id} rule={rule} agents={agents} names={names} />
          ))}
        </ul>
      )}
    </Section>
  );
}
