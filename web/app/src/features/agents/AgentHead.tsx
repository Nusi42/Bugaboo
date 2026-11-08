import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useMemo } from "react";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { usePageTrail } from "../shell/index.ts";
import { agentLine, type ListedAgent } from "./agents.ts";
import { agentAddress } from "./agent-address.ts";
import { StateButtons } from "./StateButtons.tsx";

/** The heading of the page of an agent: its name, whose it is and where it stands, *Pause* and *Stop*. */
export function AgentHead({ agent }: { readonly agent: ListedAgent }) {
  usePageTitle(agent.name);
  const trail = useMemo(
    () => [
      { label: t("agents.title"), href: paths.myAgents },
      { label: agent.name, href: agentAddress(agent.id) },
    ],
    [agent.name, agent.id],
  );
  usePageTrail(trail);
  return (
    <Card
      heading={agent.name}
      level={1}
      intro={agentLine(agent)}
      actions={<StateButtons agent={agent} />}
      back={{ href: paths.myAgents, label: t("agents.back") }}
    />
  );
}
