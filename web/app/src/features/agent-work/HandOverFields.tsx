import { t } from "@bugaboo/messages";
import { fieldKind, partOf, part, TextField } from "@bugaboo/ui";

import { AgentChoice } from "./AgentChoice.tsx";
import { BriefMissing } from "./BriefMissing.tsx";
import type { DraftProps } from "./hand-over-draft.ts";
import { HandOverLimits } from "./HandOverLimits.tsx";
import { HandOverPowers } from "./HandOverPowers.tsx";

/** What the window of a hand-over asks, top to bottom: the agent, the brief, the ticks, the budget. */
export function HandOverFields({
  draft,
  onChange,
  issues,
}: DraftProps & { readonly issues: readonly string[] }) {
  return (
    <>
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
      <p {...partOf(part.muted)}>{t("agent-work.keeps-person")}</p>
    </>
  );
}
