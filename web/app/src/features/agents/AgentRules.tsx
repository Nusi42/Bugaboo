import { t } from "@bugaboo/messages";
import { Button, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useRuleNames, useRulesOfAgents, type ListedAgent } from "./agents.ts";
import { RuleList } from "./RuleList.tsx";
import { StandingRuleDialog } from "./StandingRuleDialog.tsx";

/** The standing rules of one agent, with the button that makes one for it. */
export function AgentRules({ agent }: { readonly agent: ListedAgent }) {
  const rules = useRulesOfAgents([agent.id]);
  const names = useRuleNames(rules.rules);
  const [open, setOpen] = useState(false);
  return (
    <Card heading={t("agents.rules")}>
      <RuleList rules={rules.rules} agents={[agent]} names={names} withAgent={false} />
      {agent.state === "active" ? (
        <div {...partOf(part.row)}>
          <Button
            onPress={() => {
              setOpen(true);
            }}
          >
            {t("agents.rule-new")}
          </Button>
        </div>
      ) : null}
      {open ? (
        <StandingRuleDialog isOpen onOpenChange={setOpen} start={{ agent: agent.id }} />
      ) : null}
    </Card>
  );
}
