import type { Schema } from "@bugaboo/api-client";
import { ChoiceText, partOf, part } from "@bugaboo/ui";

import { choicesTitle, type IssueRow } from "./rows.ts";

/** The choices of a cell of the timesheet, one on each line, each as every choice is written (`4100 Product development`). */
function ChoiceList({ choices }: { readonly choices: readonly Schema<"FieldChoiceBody">[] }) {
  return choices.map((choice) => (
    <span key={choice.id} {...partOf(part.choiceLine)}>
      <ChoiceText number={choice.number} name={choice.name} />
    </span>
  ));
}

/** The *Activity* and the *Cost center* of a row, as the entries name them. */
export function FieldCells({ row }: { readonly row: IssueRow }) {
  return (
    <>
      {[row.activity, row.costCenter].map((choices, at) => (
        <td
          key={at === 0 ? "activity" : "cost-center"}
          title={choicesTitle(choices)}
          {...partOf(part.sheetName, { variants: ["value"] })}
        >
          <ChoiceList choices={choices} />
        </td>
      ))}
    </>
  );
}
