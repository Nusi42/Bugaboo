import { t } from "@bugaboo/messages";
import { ChoiceGroup, partOf, part } from "@bugaboo/ui";

import { wordParts, type CompareRow as Row, type Side, type WordPart } from "./compare.ts";

export interface CompareRowProps {
  readonly row: Row;
  /** The place in both texts, from 1. */
  readonly number: number;
  readonly side: Side;
  readonly onPick: (side: Side) => void;
}

/** A key for a stretch: its place and its words. */
function keyOfStretch(stretch: WordPart, at: number): string {
  return [at.toString(), stretch.text].join(":");
}

/** A paragraph with the words that the other text does not have marked (M3-F54). */
function Marked({ text, other }: { readonly text: string; readonly other: string }) {
  return (
    <p {...partOf(part.issueCompareParagraph)}>
      {wordParts(text, other).map((stretch, at) =>
        stretch.changed ? (
          <mark key={keyOfStretch(stretch, at)} {...partOf(part.issueCompareChanged)}>
            {stretch.text}
          </mark>
        ) : (
          stretch.text
        ),
      )}
    </p>
  );
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
            <Marked text={row.theirs} other={row.mine} />
          </div>
          <div {...partOf(part.issueCompareChoice)}>
            <span {...partOf(part.hint)}>{t("issues.compare-mine")}</span>
            <Marked text={row.mine} other={row.theirs} />
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
