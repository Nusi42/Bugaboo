import { agentStateText } from "../../shared/agent-state.ts";
import { listWord } from "../../shared/list-columns.ts";
import type { FoundLine } from "./found-issues.ts";

/** The value of the columns *Agent* and *Agent state*; none for another word. */
export function agentValueOf(word: string, line: FoundLine): string | null | undefined {
  if (word === listWord.agent) {
    return line.agent?.name;
  }
  if (word === listWord.agentState) {
    return line.agent === undefined ? undefined : agentStateText(line.agent.state);
  }
  return null;
}
