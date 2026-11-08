import { agentStateText } from "../../shared/agent-state.ts";
import { agentWords } from "../../shared/list-columns.ts";
import type { FoundLine } from "./found-issues.ts";

/** The value of the columns *Agent* and *Agent state*; none for another word. */
export function agentValueOf(word: string, line: FoundLine): string | null | undefined {
  const words = agentWords();
  if (word === words.agent) {
    return line.agent?.name;
  }
  if (word === words.state) {
    return line.agent === undefined ? undefined : agentStateText(line.agent.state);
  }
  return null;
}
