import { t } from "@bugaboo/messages";
import { Card, partOf, part, useLayout } from "@bugaboo/ui";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { usePageTrail } from "../shell/index.ts";
import { type ListedAgent } from "./agents.ts";
import { agentAddress } from "./agent-address.ts";
import { AgentStateLine } from "./AgentState.tsx";
import { OwnStateButtons } from "./StateButtons.tsx";

/**
 * The heading of the page of an agent: its name, its state pill, whose it is and the limit that
 * paused it, then *Pause* or *Resume* and *Stop*. On a computer the buttons end the heading row; on
 * a phone, where the bar holds the name, they come after the state.
 */
export function AgentHead({ agent }: { readonly agent: ListedAgent }) {
  usePageTitle(agent.name);
  const phone = useLayout() === "phone";
  const trail = useMemo(
    () => [
      { label: t("agents.title"), href: paths.myAgents },
      { label: agent.name, href: agentAddress(agent.id) },
    ],
    [agent.name, agent.id],
  );
  usePageTrail(trail);
  const buttons = <OwnStateButtons agent={agent} />;
  return (
    <Card heading={agent.name} level={1} actions={phone ? undefined : buttons}>
      <AgentStateLine agent={agent} withPerson />
      {phone ? <div {...partOf(part.row)}>{buttons}</div> : null}
    </Card>
  );
}
