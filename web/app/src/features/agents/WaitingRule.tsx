import { t } from "@bugaboo/messages";
import { Button, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { useNow } from "../../shared/use-now.ts";
import { agentNamed, ruleDaysLeft, useChangeRule, type ListedAgent, type Rule } from "./agents.ts";
import { ActionError } from "./ActionError.tsx";
import { AgentRow } from "./AgentRow.tsx";
import { type RuleNames } from "./rule-names.ts";

/**
 * A standing rule about to run out, in one sentence with the name of its saved search set apart
 * (*Sorter's standing rule* New bugs in Website *runs out in 1 day*), and *Renew* right there.
 */
export function RuleRow({
  rule,
  agents,
  names,
}: {
  readonly rule: Rule;
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
}) {
  const now = useNow();
  const renew = useChangeRule();
  const agent = agentNamed(agents, rule.agent)?.name ?? "";
  const search = names.search(rule.search);
  const sentence = t("agents.waiting-rule", {
    agent,
    search,
    count: ruleDaysLeft(rule, now),
  });
  return (
    <AgentRow>
      <span {...partOf(part.itemTitle)}>
        <SentenceWithValue sentence={sentence} value={search}>
          {(name) => <span {...partOf(part.agentsSearchName)}>{name}</span>}
        </SentenceWithValue>
      </span>
      <Button
        isPending={renew.isPending}
        aria-label={t("agents.renew-name", { agent, search })}
        onPress={() => {
          renew.mutate({
            params: { path: { rule: rule.id } },
            body: { action: "renew", version: rule.version },
          });
        }}
      >
        {t("agents.renew")}
      </Button>
      <ActionError error={renew.error} />
    </AgentRow>
  );
}
