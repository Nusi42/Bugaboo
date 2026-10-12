import { t } from "@bugaboo/messages";
import { ChoiceGroup, partOf, part } from "@bugaboo/ui";

import type { CompareRow as Row, Side } from "./compare.ts";

export interface CompareRowProps {
  readonly row: Row;
  /** The place in both texts, from 1. */
  readonly number: number;
  readonly side: Side;
  readonly onPick: (side: Side) => void;
}

/** One paragraph of *Compare and save*: the same on both sides, or the two side by side with a choice. */
export function CompareRow({ row, number, side, onPick }: CompareRowProps) {
  return (
    <fieldset {...partOf(part.issueCompareRow)}>
      <legend {...partOf(part.muted)}>{t("issues.compare-paragraph", { number })}</legend>
      {row.same ? (
        <div {...partOf(part.issueCompareChoice, { variants: ["full"] })}>
          <p {...partOf(part.issueCompareParagraph)}>{row.mine}</p>
        </div>
      ) : (
        <>
          <div {...partOf(part.issueCompareChoice)}>
            <span {...partOf(part.hint)}>{t("issues.compare-theirs")}</span>
            <p {...partOf(part.issueCompareParagraph)}>{row.theirs}</p>
          </div>
          <div {...partOf(part.issueCompareChoice)}>
            <span {...partOf(part.hint)}>{t("issues.compare-mine")}</span>
            <p {...partOf(part.issueCompareParagraph)}>{row.mine}</p>
          </div>
          <div {...partOf(part.issueCompareChoice, { variants: ["full"] })}>
            <ChoiceGroup
              legend={t("issues.compare-keep", { number })}
              choices={[
                { value: "mine", label: t("issues.compare-keep-mine") },
                { value: "theirs", label: t("issues.compare-keep-theirs") },
              ]}
              value={side}
              onChange={onPick}
            />
          </div>
        </>
      )}
    </fieldset>
  );
}
