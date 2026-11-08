import { t } from "@bugaboo/messages";
import { partOf, part, useLayout } from "@bugaboo/ui";

import { type ListedAgent, type MyWorkItem, type OfferedAgent, type Rule } from "./agents.ts";
import { AcceptedByItself } from "./AcceptedByItself.tsx";
import { AgentList } from "./AgentsSections.tsx";
import { Offered } from "./Offered.tsx";
import { useRuleNames } from "./rule-names.ts";
import { RulesSection } from "./RulesSection.tsx";
import { StopAll } from "./StopAll.tsx";
import { WaitingForYou } from "./WaitingForYou.tsx";

export interface MyAgentsBodyProps {
  readonly agents: readonly ListedAgent[];
  readonly work: readonly MyWorkItem[];
  readonly offered: readonly OfferedAgent[];
  readonly rules: readonly Rule[];
  /** Not all of the work, or not all of the agents handed over, is in the answer. */
  readonly cut: boolean;
}

/**
 * Everything below the heading: what waits, the agents, and the standing rules. On a phone *Stop all
 * my agents* ends the page, apart from *Create an agent* (UX §4.17).
 */
export function MyAgentsBody({ agents, work, offered, rules, cut }: MyAgentsBodyProps) {
  const running = agents.filter((agent) => agent.state !== "stopped");
  const names = useRuleNames(rules);
  const phone = useLayout() === "phone";
  return (
    <>
      {cut ? <p {...partOf(part.muted)}>{t("agents.cut")}</p> : null}
      <Offered offered={offered} />
      <WaitingForYou work={work} rules={rules} agents={agents} names={names} />
      <AcceptedByItself work={work} />
      <AgentList agents={agents} work={work} />
      {running.length === 0 ? null : <RulesSection agents={agents} rules={rules} names={names} />}
      {phone && running.length > 0 ? (
        <div {...partOf(part.row)}>
          <StopAll agents={running} />
        </div>
      ) : null}
    </>
  );
}
