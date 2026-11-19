import { AgentLimits, type AgentLimitsProps } from "./AgentLimits.tsx";
import { AgentSwitches } from "./AgentSwitches.tsx";

/**
 * How much an agent may do and what its person lets it do beyond its work (UX §4.17, ADR-0029,
 * amendment M9): two cards side by side on the page of the agent, each on its own.
 */
export function LimitsAndSwitches({ agent, maximums }: AgentLimitsProps) {
  return (
    <>
      <AgentLimits agent={agent} maximums={maximums} />
      <AgentSwitches agent={agent} />
    </>
  );
}
