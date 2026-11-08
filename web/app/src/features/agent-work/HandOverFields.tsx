import { t } from "@bugaboo/messages";
import { fieldKind, partOf, part, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { formatDate } from "../../shared/format.ts";
import { useIssueAgentWork } from "./agent-work.ts";
import { AgentChoice } from "./AgentChoice.tsx";
import { BriefMissing } from "./BriefMissing.tsx";
import { wholeNumber, type DraftProps } from "./hand-over-draft.ts";
import { HandOverLimits } from "./HandOverLimits.tsx";
import { HandOverPowers } from "./HandOverPowers.tsx";

const DAY_MS = 86_400_000;

/** When a hand-over made now would run out: the days typed, or the usual days of the organization. */
function EndsOn({ issue, days }: { readonly issue: string; readonly days: string }) {
  const [now] = useState(() => Date.now());
  const usual = useIssueAgentWork(issue).data?.hand_over_days;
  const counted = wholeNumber(days) ?? usual;
  if (counted === undefined) {
    return null;
  }
  const date = formatDate(new Date(now + counted * DAY_MS).toISOString());
  return <p {...partOf(part.muted)}>{t("agent-work.hand-over-ends", { date })}</p>;
}

/** What the window of a hand-over asks, top to bottom: the way it works, the agent, the brief, the ticks, the budget. */
export function HandOverFields({
  draft,
  onChange,
  issues,
  assigned,
}: DraftProps & {
  readonly issues: readonly string[];
  /** Who the issue stays assigned to, in words; none for several issues or for an issue nobody has. */
  readonly assigned: string | undefined;
}) {
  return (
    <>
      <p {...partOf(part.text)}>{t("agent-work.hand-over-text")}</p>
      <EndsOn issue={issues[0] ?? ""} days={draft.days} />
      <AgentChoice draft={draft} onChange={onChange} />
      <TextField
        label={t("agent-work.note")}
        kind={fieldKind.multiline}
        value={draft.note}
        onChange={(note) => {
          onChange({ note });
        }}
      />
      <BriefMissing issues={issues} />
      <HandOverPowers draft={draft} onChange={onChange} />
      <HandOverLimits draft={draft} onChange={onChange} />
      <p {...partOf(part.muted)}>
        {assigned === undefined
          ? t("agent-work.keeps-person")
          : t("agent-work.stays-assigned", { people: assigned })}
      </p>
    </>
  );
}
