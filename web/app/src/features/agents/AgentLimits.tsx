import { t } from "@bugaboo/messages";
import { AskDialog, Button, Card, partOf, part, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { formatNumber } from "../../shared/format.ts";
import { useChangeAgent, type ListedAgent } from "./agents.ts";
import { draftOf, limitsOf, type Limits, type LimitsDraft } from "./limits-draft.ts";

/** What the three limits are called, and where each stands in the draft. */
const FIELDS = [
  ["agents.limit-changes", "changes"],
  ["agents.limit-issues", "issues"],
  ["agents.limit-attempts", "attempts"],
] as const;

interface DialogProps {
  readonly agent: ListedAgent;
  readonly maximums: Limits;
  readonly onClose: () => void;
}

/** The window that changes the limits: the numbers can be anything from 1 to what the organization allows. */
function LimitsDialog({ agent, maximums, onClose }: DialogProps) {
  const [draft, setDraft] = useState<LimitsDraft>(() =>
    draftOf({
      changes_a_minute: agent.changesAMinute,
      issues_at_once: agent.issuesAtOnce,
      attempts_on_one_issue: agent.attemptsOnOneIssue,
    }),
  );
  const change = useChangeAgent();
  const limits = limitsOf(draft, maximums);
  const most = [maximums.changes_a_minute, maximums.issues_at_once, maximums.attempts_on_one_issue];
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("agents.limits-title", { agent: agent.name })}
      text={t("agents.limits-text")}
      confirmLabel={t("agents.limits-confirm")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={limits === undefined || change.isPending}
      confirmHint={t("agents.limits-need")}
      onConfirm={() => {
        if (limits !== undefined) {
          change.mutate({
            params: { path: { agent: agent.id } },
            body: { limits, version: agent.version },
          });
        }
      }}
    >
      {FIELDS.map(([name, key], at) => (
        <TextField
          key={key}
          label={t(name)}
          description={t("agents.limits-most", { most: most[at] ?? 0 })}
          value={draft[key]}
          onChange={(value) => {
            setDraft((was) => ({ ...was, [key]: value }));
          }}
        />
      ))}
    </AskDialog>
  );
}

export interface AgentLimitsProps {
  readonly agent: ListedAgent;
  /** The most any agent can be given; none while the page does not know it. */
  readonly maximums: Limits | null | undefined;
}

/**
 * The limits of an agent (UX §4.17, ADR-0029 decision 9): how much it may do. Reaching one pauses
 * the agent with a sentence that says which; the numbers are never more than the organization's
 * maximum, and its person can change them.
 */
export function AgentLimits({ agent, maximums }: AgentLimitsProps) {
  const [open, setOpen] = useState(false);
  const rows = [agent.changesAMinute, agent.issuesAtOnce, agent.attemptsOnOneIssue];
  return (
    <Card heading={t("agents.limits")} intro={t("agents.limits-intro")}>
      <dl {...partOf(part.agentsFacts)}>
        {FIELDS.map(([name], at) => (
          <div key={name} {...partOf(part.agentsFact)}>
            <dt {...partOf(part.muted)}>{t(name)}</dt>
            <dd {...partOf(part.agentsFactValue)}>{formatNumber(rows[at] ?? 0)}</dd>
          </div>
        ))}
      </dl>
      {maximums === undefined || maximums === null || agent.state === "stopped" ? null : (
        <div {...partOf(part.row)}>
          <Button
            onPress={() => {
              setOpen(true);
            }}
          >
            {t("agents.limits-change")}
          </Button>
          {open ? (
            <LimitsDialog
              agent={agent}
              maximums={maximums}
              onClose={() => {
                setOpen(false);
              }}
            />
          ) : null}
        </div>
      )}
    </Card>
  );
}
