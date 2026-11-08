import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { useMyAgentWork, type ListedAgent, type MyWork } from "./agents.ts";
import { WorkLines } from "./WorkLines.tsx";

/** What one agent works on now, each issue with the state of its work. */
export function AgentWorkCard({ agent }: { readonly agent: ListedAgent }) {
  const all: MyWork | undefined = useMyAgentWork().data;
  const work = all?.pieces ?? [];
  const mine = work.filter((item) => item.work.agent === agent.id && item.work.open);
  return (
    <Card heading={t("agents.work")}>
      {mine.length === 0 ? (
        <p {...partOf(part.muted)}>{t("agents.work-none")}</p>
      ) : (
        <WorkLines work={mine} />
      )}
    </Card>
  );
}
