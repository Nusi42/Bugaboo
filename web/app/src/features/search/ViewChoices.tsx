import { t } from "@bugaboo/messages";
import { ChoiceGroup, SelectField, part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { notGroupable } from "../../shared/list-columns.ts";
import { noGrouping, type ListView } from "../../shared/list-view.ts";
import { searchSpacings } from "../../shared/search-address.ts";
import type { SearchFieldsAnswer } from "./search-fields.ts";

/** How the results of a search are shown, and the way to change it. */
export interface ListViewControl {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
}

export interface ViewChoicesProps extends ListViewControl {
  readonly fields: SearchFieldsAnswer;
  /** What stands with *Spacing*, on the same line (the button *Customize* on a computer). */
  readonly end?: ReactNode;
}

/** The fields a list can be grouped by: every one the person may read. */
function groupChoicesOf(fields: SearchFieldsAnswer) {
  return Object.keys(fields.names)
    .filter((word) => !notGroupable.has(word))
    .map((word) => ({ value: word, label: fields.names[word] ?? word }));
}

/**
 * *Group by* any field the person may read, and *Spacing* (UX section 4.2): above the list on a
 * computer, in the *Filters* sheet on a phone (M5 exit decision D2).
 */
export function ViewChoices({ view, fields, onView, end }: ViewChoicesProps) {
  return (
    <>
      <SelectField
        label={t("issues.group-by")}
        choices={[
          { value: noGrouping, label: t("issues.group-by-none") },
          ...groupChoicesOf(fields),
        ]}
        value={view.groupBy ?? noGrouping}
        onChange={(value) => {
          onView({ ...view, groupBy: value === noGrouping ? null : value });
        }}
      />
      <div {...partOf(part.row, { variants: ["nowrap"] })}>
        <ChoiceGroup
          legend={t("appearance.spacing.legend")}
          choices={searchSpacings.map((word) => ({
            value: word,
            label: t(`appearance.spacing.${word}`),
          }))}
          value={view.spacing}
          onChange={(spacing) => {
            onView({ ...view, spacing });
          }}
        />
        {end}
      </div>
    </>
  );
}
