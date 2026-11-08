import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part, useLayout } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import {
  listedAgents,
  useMyAgentList,
  useMyAgentWork,
  useRuleNames,
  useRulesOfAgents,
  type ListedAgent,
  type OfferedAgent,
  type MyWork,
  type MyWorkItem,
} from "./agents.ts";
import { AcceptedByItself } from "./AcceptedByItself.tsx";
import { AgentList, RulesSection } from "./AgentsSections.tsx";
import { Offered } from "./Offered.tsx";
import { StopAll } from "./StopAll.tsx";
import { WaitingForYou } from "./WaitingForYou.tsx";

interface BodyProps {
  readonly agents: readonly ListedAgent[];
  readonly work: readonly MyWorkItem[];
  readonly offered: readonly OfferedAgent[];
}

/** Everything below the heading: what waits, the agents, and the standing rules. */
function Body({ agents, work, offered }: BodyProps) {
  const running = agents.filter((agent) => agent.state !== "stopped");
  const lists = useRulesOfAgents(running.map((agent) => agent.id));
  const names = useRuleNames(lists.rules);
  return (
    <>
      <Offered offered={offered} />
      <WaitingForYou work={work} rules={lists.rules} agents={agents} names={names} />
      <AcceptedByItself work={work} />
      <AgentList agents={agents} work={work} />
      {running.length === 0 ? null : (
        <RulesSection agents={agents} rules={lists.rules} names={names} />
      )}
    </>
  );
}

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
          <Body agents={loaded.listed} work={loaded.work.pieces} offered={loaded.work.offered} />
        )}
      </QueryGate>
    </div>
  );
}
