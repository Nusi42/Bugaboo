import { t } from "@bugaboo/messages";
import { ChoiceGroup, fieldKind, SelectField, TextField } from "@bugaboo/ui";

import type { Work } from "./agent-work.ts";
import { KEEP_STATUS, type ReviewDraft } from "./use-review-draft.ts";
import { ThrowAwayPreview } from "./ThrowAwayPreview.tsx";

/** The answer to choose and what each answer still needs: a status, a note, or what would go back. */
export function ReviewFields({
  work,
  draft,
}: {
  readonly work: Work;
  readonly draft: ReviewDraft;
}) {
  return (
    <>
      <ChoiceGroup
        legend={t("agent-work.review-choose")}
        value={draft.answer}
        onChange={draft.setAnswer}
        choices={[
          { value: "accept", label: t("agent-work.review-accept-choice") },
          { value: "send-back", label: t("agent-work.review-send-back-choice") },
          { value: "throw-away", label: t("agent-work.review-throw-away-choice"), danger: true },
        ]}
      />
      {draft.answer === "accept" && draft.choices.length > 0 ? (
        <SelectField
          label={t("agent-work.review-status")}
          value={draft.status}
          onChange={draft.setStatus}
          choices={[
            ...draft.choices.map((choice) => ({
              value: choice.value,
              label: choice.proposed
                ? t("agent-work.review-status-proposed", { status: choice.label })
                : choice.label,
            })),
            { value: KEEP_STATUS, label: t("agent-work.review-status-keep") },
          ]}
        />
      ) : null}
      {draft.answer === "send-back" ? (
        <TextField
          label={t("agent-work.review-note-label")}
          description={t("agent-work.review-send-back-help")}
          kind={fieldKind.multiline}
          value={draft.note}
          onChange={draft.setNote}
        />
      ) : null}
      {draft.answer === "throw-away" ? <ThrowAwayPreview work={work} /> : null}
    </>
  );
}
