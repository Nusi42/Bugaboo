import { t } from "@bugaboo/messages";
import { SelectField, selectLook, type SelectLook } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { notGroupable } from "../../shared/list-columns.ts";
import { noGrouping, type ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "./search-fields.ts";

/** How the results of a search are shown, and the way to change it. */
export interface ListViewControl {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
}

export interface ViewChoicesProps extends ListViewControl {
  readonly fields: SearchFieldsAnswer;
  /** What stands after *Group by* (the button *Customize* on a computer). */
  readonly end?: ReactNode;
  /** How *Group by* looks: quiet words above a list on a computer, a box in the *Filters* sheet. */
  readonly look?: SelectLook;
}

/** The fields a list can be grouped by: every one the person may read. */
function groupChoicesOf(fields: SearchFieldsAnswer) {
  return Object.keys(fields.names)
    .filter((word) => !notGroupable.has(word))
    .map((word) => ({ value: word, label: fields.names[word] ?? word }));
}

/**
 * *Group by* any field the person may read (UX section 4.2): above the list on a computer, in the
 * *Filters* sheet on a phone (M5 exit decision D2). *Spacing* is no choice of a list: every screen
 * follows the person's *Spacing* in *My settings › Appearance* (ADR-0034, amendment).
 */
export function ViewChoices({
  view,
  fields,
  onView,
  end,
  look = selectLook.box,
}: ViewChoicesProps) {
  return (
    <>
      <SelectField
        label={t("issues.group-by")}
        look={look}
        choices={[
          { value: noGrouping, label: t("issues.group-by-none") },
          ...groupChoicesOf(fields),
        ]}
        value={view.groupBy ?? noGrouping}
        onChange={(value) => {
          onView({ ...view, groupBy: value === noGrouping ? null : value });
        }}
      />
      {end}
    </>
  );
}
