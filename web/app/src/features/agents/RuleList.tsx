import { t } from "@bugaboo/messages";
import { Button, buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { useNow } from "../../shared/use-now.ts";
import { ActionError } from "./ActionError.tsx";
import { agentNamed, runsOutText, useChangeRule, type ListedAgent, type Rule } from "./agents.ts";
import { type RuleNames } from "./rule-names.ts";
import { AgentRow } from "./AgentRow.tsx";

/** What a rule does, in words: it proposes, or it hands over what it finds, and it may create issues in a project. */
function powerText(rule: Rule, names: RuleNames): string {
  const power =
    rule.way === "hand-over" ? t("agents.rule-power-hand-over") : t("agents.rule-power-propose");
  return rule.creates_in === undefined || rule.creates_in === null
    ? power
    : `${power} ${t("agents.rule-power-creates", { project: names.project(rule.creates_in) })}`;
}

/** The strong settings of a rule that hands over: what its hand-overs may do besides what an agent always may, and accepting by itself. */
function HandOverFacts({ rule }: { readonly rule: Rule }) {
  if (rule.way !== "hand-over") {
    return null;
  }
  const powers = rule.powers.map((power) => t(`agent-work.power-${power}`)).join(", ");
  return (
    <>
      {powers === "" ? null : (
        <span {...partOf(part.muted)}>{t("agents.rule-powers", { powers })}</span>
      )}
      {rule.accept_by_itself ? (
        <span {...partOf(part.muted)}>{t("agents.rule-accepts")}</span>
      ) : null}
    </>
  );
}

interface RuleRowProps {
  readonly rule: Rule;
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
  /** The agent is the heading of the list already (the page of an agent): its name is not said again. */
  readonly withAgent: boolean;
  /** The reader is the agent's person: only they renew and stop a rule. */
  readonly canChange: boolean;
}

/** *Renew* and *Stop* of a rule that works. */
function RuleActions({ rule, agent, search }: ActionsProps) {
  const change = useChangeRule();
  const send = (action: "renew" | "stop"): void => {
    change.mutate({
      params: { path: { rule: rule.id } },
      body: { action, version: rule.version },
    });
  };
  return (
    <div {...partOf(part.agentsActions)}>
      {rule.ends_at === undefined || rule.ends_at === null ? null : (
        <Button
          isPending={change.isPending}
          aria-label={t("agents.renew-name", { agent, search })}
          onPress={() => {
            send("renew");
          }}
        >
          {t("agents.renew")}
        </Button>
      )}
      <ConfirmDialog
        triggerLabel={t("agents.rule-stop")}
        triggerName={t("agents.rule-stop-name", { agent, search })}
        triggerLook={buttonLook.danger}
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
  );
}

interface ActionsProps {
  readonly rule: Rule;
  readonly agent: string;
  readonly search: string;
}

/** One standing rule: the search, the agent, what it may do, when it runs out, *Renew* and *Stop*. */
function RuleRow({ rule, agents, names, withAgent, canChange }: RuleRowProps) {
  const now = useNow();
  const agent = agentNamed(agents, rule.agent)?.name ?? "";
  const search = names.search(rule.search);
  return (
    <AgentRow>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          {withAgent ? t("agents.rule-line", { search, agent }) : search}
        </span>
        <span {...partOf(part.muted)}>{powerText(rule, names)}</span>
        <HandOverFacts rule={rule} />
        <span
          {...partOf(
            rule.state === "open" && (rule.ends_at === undefined || rule.ends_at === null)
              ? part.agentsNeverEnds
              : part.muted,
          )}
        >
          {rule.state === "ended" ? t("agents.rule-ended") : runsOutText(rule.ends_at, now)}
        </span>
      </span>
      {rule.state === "ended" || !canChange ? null : (
        <RuleActions rule={rule} agent={agent} search={search} />
      )}
    </AgentRow>
  );
}

export interface RuleListProps {
  readonly rules: readonly Rule[];
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
  readonly withAgent?: boolean;
  /** Whether the reader may renew and stop the rules; they may by default. */
  readonly canChange?: boolean;
}

/** The standing rules: those that work with *Renew* and *Stop*, those that ended as they ended (UX §4.17). */
export function RuleList({
  rules,
  agents,
  names,
  withAgent = true,
  canChange = true,
}: RuleListProps) {
  if (rules.length === 0) {
    return <p {...partOf(part.muted)}>{t("agents.rules-none")}</p>;
  }
  return (
    <ul {...partOf(part.list)}>
      {rules.map((rule) => (
        <RuleRow
          key={rule.id}
          rule={rule}
          agents={agents}
          names={names}
          withAgent={withAgent}
          canChange={canChange}
        />
      ))}
    </ul>
  );
}
