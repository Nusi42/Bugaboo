import { AgentHistory } from "./AgentHistory.tsx";
import { AgentRules } from "./AgentRules.tsx";
import { AgentWorkCard } from "./AgentWorkCard.tsx";
import type { ListedAgent } from "./agents.ts";

/**
 * What an agent works on, its standing rules, and for its person its *History*: what it handed over
 * and what it undid (UX §4.17, ADR-0029, amendment M9). Three cards on the page of the agent.
 */
export function WorkRulesAndHistory({ agent }: { readonly agent: ListedAgent }) {
  return (
    <>
      <AgentWorkCard agent={agent} />
      <AgentRules agent={agent} />
      {agent.yours ? <AgentHistory agent={agent} /> : null}
    </>
  );
}
