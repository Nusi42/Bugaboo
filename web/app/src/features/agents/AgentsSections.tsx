import { t } from "@bugaboo/messages";
import { Button, partOf, part, useLayout } from "@bugaboo/ui";
import { useState } from "react";

import { type ListedAgent, type MyWorkItem, type Rule, type RuleNames } from "./agents.ts";
import { AgentSection } from "./AgentSection.tsx";
import { CreateAgentButton } from "./CreateAgentDialog.tsx";
import { RuleList } from "./RuleList.tsx";
import { Section } from "./Section.tsx";
import { StandingRuleDialog } from "./StandingRuleDialog.tsx";
import { StopAll } from "./StopAll.tsx";

export interface AgentsSectionsProps {
  readonly agents: readonly ListedAgent[];
  readonly work: readonly MyWorkItem[];
}

/** *Agents*: one box for each agent, and the buttons to make one (and, on a phone, to stop them all). */
export function AgentList({ agents, work }: AgentsSectionsProps) {
  const phone = useLayout() === "phone";
  const running = agents.filter((agent) => agent.state !== "stopped");
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
        {phone && running.length > 0 ? <StopAll agents={running} /> : null}
      </div>
    </Section>
  );
}

export interface RulesSectionProps {
  readonly agents: readonly ListedAgent[];
  readonly rules: readonly Rule[];
  readonly names: RuleNames;
}

/** *Standing rules*: the ones that work, with *Renew* and *Stop*, and the button that makes one. */
export function RulesSection({ agents, rules, names }: RulesSectionProps) {
  const [open, setOpen] = useState(false);
  return (
    <Section
      heading={t("agents.rules")}
      count={rules.filter((rule) => rule.state === "open").length}
    >
      <RuleList rules={rules} agents={agents} names={names} />
      <div {...partOf(part.row)}>
        <Button
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("agents.rule-new")}
        </Button>
      </div>
      {open ? <StandingRuleDialog isOpen onOpenChange={setOpen} /> : null}
    </Section>
  );
}
