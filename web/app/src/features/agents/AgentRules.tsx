import { t } from "@bugaboo/messages";
import { Button, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useAgentRules, type ListedAgent, type RulePage } from "./agents.ts";
import { useRuleNames } from "./rule-names.ts";
import { RuleList } from "./RuleList.tsx";
import { StandingRuleDialog } from "./StandingRuleDialog.tsx";

/** The standing rules of one agent, with the button that makes one for it. */
export function AgentRules({ agent }: { readonly agent: ListedAgent }) {
  const found: RulePage | undefined = useAgentRules(agent.id).data;
  const mine = found?.items ?? [];
  const names = useRuleNames(mine);
  const [open, setOpen] = useState(false);
  return (
    <Card heading={t("agents.rules")}>
      <RuleList rules={mine} agents={[agent]} names={names} withAgent={false} />
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
