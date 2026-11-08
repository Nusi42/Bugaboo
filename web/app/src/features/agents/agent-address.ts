import { paths } from "../../shared/paths.ts";

/** The address of the page of an agent, `/me/agents/<ID>`: its keys, limits, work and rules. */
export function agentAddress(id: string): string {
  return paths.agent.replace("$agent", encodeURIComponent(id));
}
