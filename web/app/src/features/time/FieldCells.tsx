import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ChoiceText, partOf, part } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";

import { choicesTitle, type IssueRow } from "./rows.ts";
import { gridLength } from "./sheet-model.ts";

/** The choices of a cell of the timesheet, one on each line, each as every choice is written (`4100 Product development`). */
function ChoiceList({
  choices,
  minutes,
  wording,
}: {
  readonly choices: readonly Schema<"FieldChoiceBody">[];
  readonly wording: Wording;
  /** The minutes of each choice, said after it when the cell holds more than one: which length is whose. */
  readonly minutes: Readonly<Record<string, number>>;
}) {
  return choices.map((choice) => (
    <span key={choice.id} {...partOf(part.choiceLine)}>
      <ChoiceText number={choice.number} name={choice.name} />
      {choices.length > 1 ? (
        <span {...partOf(part.muted)}>
          {t("time.sheet.facts-separator")}
          {gridLength(wording, minutes[choice.id] ?? 0)}
        </span>
      ) : null}
    </span>
  ));
}

/** The *Activity* and the *Cost center* of a row, as the entries name them. */
export function FieldCells({
  row,
  wording,
}: {
  readonly row: IssueRow;
  readonly wording: Wording;
}) {
  return (
    <>
      {[row.activity, row.costCenter].map((choices, at) => (
        <td
          key={at === 0 ? "activity" : "cost-center"}
          title={choicesTitle(choices)}
          {...partOf(part.sheetName, { variants: ["value"] })}
        >
          <ChoiceList choices={choices} minutes={row.choiceMinutes} wording={wording} />
        </td>
      ))}
    </>
  );
}
