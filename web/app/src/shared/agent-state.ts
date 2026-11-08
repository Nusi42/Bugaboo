import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** Where the work of an agent on an issue is, by the API's own word. It is not the status of the issue. */
export type WorkState = Schema<"AgentWorkState">;

/** Where the work is, in words. The words come from the catalog by the API's own word. */
export function agentStateText(state: WorkState): string {
  return t(`agent-work.state.${state}`);
}
