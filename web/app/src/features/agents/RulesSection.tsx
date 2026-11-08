import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { type ListedAgent, type Rule } from "./agents.ts";
import { type RuleNames } from "./rule-names.ts";
import { RuleList } from "./RuleList.tsx";
import { Section } from "./Section.tsx";
import { StandingRuleDialog } from "./StandingRuleDialog.tsx";

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
