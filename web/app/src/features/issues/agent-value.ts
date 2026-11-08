import { agentStateText } from "../../shared/agent-state.ts";
import { listWord } from "../../shared/list-columns.ts";
import type { FoundLine } from "./found-issues.ts";

/** Whether the word names the column *Agent* or *Agent state*, which the line of the issue answers. */
export function isAgentWord(word: string): boolean {
  return word === listWord.agent || word === listWord.agentState;
}

/** The value of the column *Agent* or *Agent state* for the agent of a line; none where no agent works on the issue. */
export function agentValueOf(word: string, agent?: FoundLine["agent"]): string | undefined {
  if (agent === undefined) {
    return undefined;
  }
  return word === listWord.agent ? agent.name : agentStateText(agent.state);
}
