import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { useNow } from "../../shared/use-now.ts";
import { ActionError } from "./ActionError.tsx";
import { agentNamed, runsOutText, useChangeRule, type ListedAgent, type Rule } from "./agents.ts";
import { type RuleNames } from "./rule-names.ts";
import { ruleFieldText } from "./rule-draft.ts";

/** What a rule may do, in words: it proposes, or it changes these fields, and it may create issues in a project. */
function powerText(rule: Rule, names: RuleNames): string {
  const power =
    rule.fields.length === 0
      ? t("agents.rule-power-propose")
      : t("agents.rule-power-change", { fields: rule.fields.map(ruleFieldText).join(", ") });
  return rule.creates_in === undefined || rule.creates_in === null
    ? power
    : `${power} ${t("agents.rule-power-creates", { project: names.project(rule.creates_in) })}`;
}

interface RuleRowProps {
  readonly rule: Rule;
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
  /** The agent is the heading of the list already (the page of an agent): its name is not said again. */
  readonly withAgent: boolean;
}

/** One standing rule: the search, the agent, what it may do, when it runs out, *Renew* and *Stop*. */
function RuleRow({ rule, agents, names, withAgent }: RuleRowProps) {
  const now = useNow();
  const change = useChangeRule();
  const agent = agentNamed(agents, rule.agent)?.name ?? "";
  const search = names.search(rule.search);
  const send = (action: "renew" | "stop"): void => {
    change.mutate({
      params: { path: { rule: rule.id } },
      body: { action, version: rule.version },
    });
  };
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          {withAgent ? t("agents.rule-line", { search, agent }) : search}
        </span>
        <span {...partOf(part.muted)}>{powerText(rule, names)}</span>
        <span {...partOf(part.muted)}>{runsOutText(rule.ends_at, now)}</span>
      </span>
      <div {...partOf(part.agentsActions)}>
        <Button
          isPending={change.isPending}
          aria-label={t("agents.renew-name", { agent, search })}
          onPress={() => {
            send("renew");
          }}
        >
          {t("agents.renew")}
        </Button>
        <ConfirmDialog
          triggerLabel={t("agents.rule-stop")}
          triggerName={t("agents.rule-stop-name", { agent, search })}
          title={t("agents.rule-stop-title")}
          text={t("agents.rule-stop-text")}
          confirmLabel={t("agents.rule-stop")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          onConfirm={() => {
            send("stop");
          }}
        />
        <ActionError error={change.error} />
      </div>
    </li>
  );
}

export interface RuleListProps {
  readonly rules: readonly Rule[];
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
  readonly withAgent?: boolean;
}

/** The standing rules that still work, with *Renew* and *Stop* (UX §4.17). */
export function RuleList({ rules, agents, names, withAgent = true }: RuleListProps) {
  const working = rules.filter((rule) => rule.state === "open");
  if (working.length === 0) {
    return <p {...partOf(part.muted)}>{t("agents.rules-none")}</p>;
  }
  return (
    <ul {...partOf(part.list)}>
      {working.map((rule) => (
        <RuleRow key={rule.id} rule={rule} agents={agents} names={names} withAgent={withAgent} />
      ))}
    </ul>
  );
}
