import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part, useLayout } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { listedAgents, useMyAgentList, useMyAgentWork, type MyWork } from "./agents.ts";
import { MyAgentsBody } from "./MyAgentsBody.tsx";
import { StopAll } from "./StopAll.tsx";

/**
 * *My agents* (UX §4.17, ADR-0029 decision 8): what waits for the person first, then each agent with
 * what it works on, then the standing rules. On a phone *Waiting for you* is first, the agents
 * below, and the same buttons are 44 px high.
 */
export function MyAgentsPage() {
  usePageTitle(t("agents.title"));
  const agents = useMyAgentList();
  const work = useMyAgentWork();
  const mine: MyWork | undefined = work.data;
  const phone = useLayout() === "phone";
  const listed = agents.data === undefined ? undefined : listedAgents(agents.data.items);
  const running = (listed ?? []).filter((agent) => agent.state !== "stopped");
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={t("agents.title")}
        level={1}
        look={cardLook.pageHead}
        intro={t("agents.intro")}
        actions={phone || running.length === 0 ? undefined : <StopAll agents={running} />}
      />
      <QueryGate
        error={agents.error ?? work.error}
        data={listed === undefined || mine === undefined ? undefined : { listed, work: mine }}
      >
        {(loaded) => (
          <MyAgentsBody
            agents={loaded.listed}
            work={loaded.work.pieces}
            offered={loaded.work.offered}
            rules={loaded.work.rules}
            cut={
              loaded.work.truncated || loaded.work.offered_truncated || loaded.work.rules_truncated
            }
          />
        )}
      </QueryGate>
    </div>
  );
}
