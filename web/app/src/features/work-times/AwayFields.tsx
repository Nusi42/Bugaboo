import { t } from "@bugaboo/messages";
import { ChoiceGroup, TextField } from "@bugaboo/ui";

import { DayBox } from "../../shared/DayBox.tsx";
import type { AwayDraft, HalfChoice } from "./away-draft.ts";

/** A day of the form: its box, and whether it is whole or half. */
function DayWithHalf({
  label,
  day,
  onDay,
  legend,
  half,
  halfLabel,
  onHalf,
}: {
  readonly label: string;
  readonly day: string;
  readonly onDay: (day: string) => void;
  readonly legend: string;
  readonly half: HalfChoice;
  readonly halfLabel: string;
  readonly onHalf: (choice: HalfChoice) => void;
}) {
  return (
    <>
      <DayBox label={label} value={day} onChange={onDay} />
      <ChoiceGroup
        legend={legend}
        choices={[
          { value: "whole", label: t("work-times.whole-day") },
          { value: "half", label: halfLabel },
        ]}
        value={half}
        onChange={onHalf}
      />
    </>
  );
}

/**
 * The boxes of an away time: *From* with the first day whole or *Afternoon only*, *To* with the last
 * day whole or *Morning only*, and the note that only its person sees (none for somebody else).
 */
export function AwayFields({
  draft,
  withNote,
}: {
  readonly draft: AwayDraft;
  readonly withNote: boolean;
}) {
  return (
    <>
      <DayWithHalf
        label={t("work-times.from")}
        day={draft.first}
        onDay={draft.setFirst}
        legend={t("work-times.first-day")}
        half={draft.firstHalf}
        halfLabel={t("work-times.afternoon-only")}
        onHalf={draft.setFirstHalf}
      />
      <DayWithHalf
        label={t("work-times.to")}
        day={draft.last}
        onDay={draft.setLast}
        legend={t("work-times.last-day")}
        half={draft.lastHalf}
        halfLabel={t("work-times.morning-only")}
        onHalf={draft.setLastHalf}
      />
      {withNote ? (
        <TextField label={t("work-times.note")} value={draft.note} onChange={draft.setNote} />
      ) : null}
    </>
  );
}
