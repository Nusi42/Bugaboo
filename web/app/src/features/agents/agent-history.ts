import type { ReadonlyDeep, Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { listText } from "../../shared/list-text.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { agentKeys } from "./agent-keys.ts";

/** One entry of the history of an agent: a hand-over to another agent, or an undo of one of its runs. */
export type AgentDeed = ReadonlyDeep<Schema<"AgentHistoryItemBody">>;

/** What one agent handed over and undid, newest first, a page at a time. */
export function useAgentHistory(agent: string) {
  return usePaged({
    queryKey: agentKeys.history(agent),
    read: (after) =>
      api.client.GET("/api/v1/agents/{agent}/history", {
        params: { path: { agent }, query: { limit: pageSize.short, ...after } },
      }),
  });
}

/** What one entry of an agent's history says, and the readable IDs of the issues in it. */
export interface DeedWords {
  readonly sentence: string;
  readonly issues: readonly string[];
}

/** The sentence of one entry of the history of an agent: a hand-over to another agent, or an undo of one of its runs. */
export function deedWords(deed: AgentDeed): DeedWords {
  const issues = deed.issues.map((issue) => issue.readable_id);
  if (deed.kind === "undo") {
    const count = deed.went_back ?? 0;
    const issue = issues[0];
    return {
      sentence:
        issue === undefined
          ? t("agents.history-undid-unseen", { count })
          : t("agents.history-undid", { issue, count }),
      issues,
    };
  }
  const agent = deed.to?.name ?? t("agents.history-another-agent");
  return {
    sentence:
      issues.length === 0
        ? t("agents.history-handed-over-unseen", { agent })
        : t("agents.history-handed-over", { issues: listText(issues), agent }),
    issues,
  };
}
