import { t } from "@bugaboo/messages";
import { IssueId, Link, partOf, part } from "@bugaboo/ui";

import { AgentMark } from "../../shared/AgentMark.tsx";
import { issueAddress } from "../../shared/paths.ts";
import type { MyWorkItem } from "./agents.ts";

/** What an agent works on: each issue with the state of its work, as the mark and the words. */
export function WorkLines({ work }: { readonly work: readonly MyWorkItem[] }) {
  return (
    <ul {...partOf(part.agentsWork)}>
      {work.map((item) => (
        <li key={item.work.id} {...partOf(part.agentsWorkLine)}>
          <Link href={issueAddress(item.issue.readable_id)}>
            <IssueId id={item.issue.readable_id} /> {item.issue.title ?? ""}
          </Link>
          <span {...partOf(part.muted)}>
            {t("agents.work-state", { state: t(`agent-work.state.${item.work.state}`) })}
            <AgentMark name={item.work.agent_name} state={item.work.state} />
          </span>
        </li>
      ))}
    </ul>
  );
}
