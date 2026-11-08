import { t } from "@bugaboo/messages";
import { NameValueRow } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";
import { useIssueAgentWork } from "./agent-work.ts";

/**
 * *Worked on by Fixer* next to *Assigned to* in the Details (AI-33, UX §4.17): the agents with open
 * work on the issue, each with the person it works for. The issue keeps its person; an agent is
 * never *Assigned to*. Without open work there is no row.
 */
export function WorkedOnByRow({ readableId }: { readonly readableId: string }) {
  const found = useIssueAgentWork(readableId);
  const agents = found.data?.worked_on_by ?? [];
  if (agents.length === 0) {
    return null;
  }
  return (
    <NameValueRow name={t("agent-work.worked-on-by")}>
      {agents.map((agent) => (
        <div key={agent.agent}>
          {t("agent-work.agent-of", { agent: agent.name, person: personName(agent.person) })}
        </div>
      ))}
    </NameValueRow>
  );
}
