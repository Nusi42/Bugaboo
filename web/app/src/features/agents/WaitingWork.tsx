import { t } from "@bugaboo/messages";
import { Button, IssueId, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress } from "../../shared/paths.ts";
import { agentNamed, type ListedAgent, type MyWorkItem } from "./agents.ts";
import { AgentRow } from "./AgentRow.tsx";

/** What a row of *Waiting for you* says and what its button is called, by what the work waits for. */
const WAITS = {
  question: { line: "agents.waiting-question", button: "agents.answer" },
  plan: { line: "agents.waiting-plan", button: "agent-work.plan-approve" },
  review: { line: "agents.waiting-review", button: "agents.review" },
} as const;

/** One question, plan or result: the issue, the agent, what waits, and a button to where it is answered. */
export function WorkRow({
  item,
  agents,
}: {
  readonly item: MyWorkItem;
  readonly agents: readonly ListedAgent[];
}) {
  const navigate = useNavigate();
  const waits = item.waits_for;
  if (waits === undefined || waits === null) {
    return null;
  }
  const words = WAITS[waits];
  const agent = agentNamed(agents, item.work.agent)?.name ?? item.work.agent_name;
  const key = item.issue.readable_id;
  return (
    <AgentRow>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          <IssueId id={key} /> {item.issue.title ?? ""}
        </span>
        <span {...partOf(part.muted)}>{t(words.line, { agent })}</span>
      </span>
      <Button
        aria-label={t("agents.waiting-button-name", { action: t(words.button), issue: key })}
        onPress={() => {
          void navigate({ href: issueAddress(key) });
        }}
      >
        {t(words.button)}
      </Button>
    </AgentRow>
  );
}
