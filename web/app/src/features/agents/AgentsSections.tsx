import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { type ListedAgent, type MyWorkItem } from "./agents.ts";
import { AgentSection } from "./AgentSection.tsx";
import { CreateAgentButton } from "./CreateAgentDialog.tsx";
import { Section } from "./Section.tsx";

export interface AgentsSectionsProps {
  readonly agents: readonly ListedAgent[];
  readonly work: readonly MyWorkItem[];
}

/** *Agents*: one box for each agent, in the order they come in, and the button to make one. */
export function AgentList({ agents, work }: AgentsSectionsProps) {
  return (
    <Section heading={t("agents.list")} count={agents.length}>
      {agents.length === 0 ? (
        <p {...partOf(part.muted)}>{t("agents.none")}</p>
      ) : (
        <ul {...partOf(part.list)}>
          {agents.map((agent) => (
            <AgentSection
              key={agent.id}
              agent={agent}
              work={work.filter((item) => item.work.agent === agent.id)}
            />
          ))}
        </ul>
      )}
      <div {...partOf(part.row)}>
        <CreateAgentButton count={agents.length} />
      </div>
    </Section>
  );
}
