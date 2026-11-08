import { t } from "@bugaboo/messages";
import { Button, buttonLook, IssueId, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress } from "../../shared/paths.ts";
import { useNow } from "../../shared/use-now.ts";
import {
  agentNamed,
  runsOutSoon,
  runsOutText,
  useChangeRule,
  type ListedAgent,
  type MyWorkItem,
  type Rule,
  type RuleNames,
} from "./agents.ts";
import { Section } from "./Section.tsx";

/** What a row of *Waiting for you* says and what its button is called, by what the work waits for. */
const WAITS = {
  question: { line: "agents.waiting-question", button: "agents.answer" },
  plan: { line: "agents.waiting-plan", button: "agents.approve" },
  review: { line: "agents.waiting-review", button: "agents.review" },
} as const;

/** One question, plan or result: the issue, the agent, what waits, and a button to where it is answered. */
function WorkRow({
  item,
  agents,
}: {
  readonly item: MyWorkItem;
  readonly agents: readonly ListedAgent[];
}) {
  const navigate = useNavigate();
  const waits = item.waits_for;
  if (waits === undefined || waits === null) {
    return null;
  }
  const words = WAITS[waits];
  const agent = agentNamed(agents, item.work.agent)?.name ?? item.work.agent_name;
  const key = item.issue.readable_id;
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          <IssueId id={key} /> {item.issue.title ?? ""}
        </span>
        <span {...partOf(part.muted)}>{t(words.line, { agent })}</span>
      </span>
      <Button
        look={buttonLook.primary}
        aria-label={t("agents.waiting-button-name", { action: t(words.button), issue: key })}
        onPress={() => {
          void navigate({ href: issueAddress(key) });
        }}
      >
        {t(words.button)}
      </Button>
    </li>
  );
}

/** A standing rule about to run out, with *Renew* right there. */
function RuleRow({
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
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          {t("agents.waiting-rule", { agent, search: names.search(rule.search) })}
        </span>
        <span {...partOf(part.muted)}>{runsOutText(rule.ends_at, now)}</span>
      </span>
      <Button
        look={buttonLook.primary}
        isPending={renew.isPending}
        aria-label={t("agents.renew-name", { agent, search: names.search(rule.search) })}
        onPress={() => {
          renew.mutate({
            params: { path: { rule: rule.id } },
            body: { action: "renew", version: rule.version },
          });
        }}
      >
        {t("agents.renew")}
      </Button>
    </li>
  );
}

export interface WaitingForYouProps {
  readonly work: readonly MyWorkItem[];
  readonly rules: readonly Rule[];
  readonly agents: readonly ListedAgent[];
  readonly names: RuleNames;
}

/**
 * *Waiting for you* (UX §4.17): the questions, the plans to approve, the results to review and the
 * standing rules about to run out, each with a button to the place where it is answered.
 */
export function WaitingForYou({ work, rules, agents, names }: WaitingForYouProps) {
  const now = useNow();
  const waiting = work.filter((item) => item.waits_for !== undefined && item.waits_for !== null);
  const ending = rules.filter((rule) => runsOutSoon(rule, now));
  const count = waiting.length + ending.length;
  return (
    <Section heading={t("agents.waiting")} count={count}>
      {count === 0 ? (
        <p {...partOf(part.muted)}>{t("agents.waiting-none")}</p>
      ) : (
        <ul {...partOf(part.list)}>
          {waiting.map((item) => (
            <WorkRow key={item.work.id} item={item} agents={agents} />
          ))}
          {ending.map((rule) => (
            <RuleRow key={rule.id} rule={rule} agents={agents} names={names} />
          ))}
        </ul>
      )}
    </Section>
  );
}
