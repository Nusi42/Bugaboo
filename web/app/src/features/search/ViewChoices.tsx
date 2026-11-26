import { t } from "@bugaboo/messages";
import { SelectField, selectLook, type SelectLook } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { notGroupable } from "../../shared/list-columns.ts";
import {
  noGrouping,
  timelineStartGroup,
  type ListView,
  type ShownAs,
} from "../../shared/list-view.ts";
import { timelineGroupable, type SearchFieldsAnswer } from "./search-fields.ts";

/** How the results of a search are shown, and the way to change it. */
export interface ListViewControl {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
}

export interface ViewChoicesProps extends ListViewControl {
  readonly fields: Pick<SearchFieldsAnswer, "names" | "timelineGroups">;
  /** What stands after *Group by* (the button *Customize* on a computer). */
  readonly end?: ReactNode;
  /** How *Group by* looks: quiet words above a list on a computer, a box in the *Filters* sheet. */
  readonly look?: SelectLook;
}

/** A list can be grouped by every field the person may read but a long text and the Issue ID. */
function listGroupable(word: string): boolean {
  return !notGroupable.has(word);
}

/** The fields the results of a view of this kind can be grouped by, by their words. */
function groupableOf(kind: ShownAs, fields: Pick<SearchFieldsAnswer, "timelineGroups">) {
  return kind === "timeline" ? timelineGroupable(fields) : listGroupable;
}

/** The fields the results can be grouped by. */
function groupChoicesOf(
  fields: Pick<SearchFieldsAnswer, "names">,
  groupable: (word: string) => boolean,
) {
  return Object.keys(fields.names)
    .filter((word) => groupable(word))
    .map((word) => ({ value: word, label: fields.names[word] ?? word }));
}

/** The ways to show the results (*Show as*, UX §2.1), with the key of each name. */
const shownAsChoices = [
  { value: "list", label: "timeline.show-as-list" },
  { value: "board", label: "board.show-as-board" },
  { value: "timeline", label: "timeline.show-as-timeline" },
  { value: "map", label: "timeline.show-as-map" },
] as const satisfies readonly { readonly value: ShownAs; readonly label: string }[];

/**
 * *Show as* and *Group by* any field the person may read (UX section 4.2): above the list on a
 * computer, in the *Filters* sheet on a phone (M5 exit decision D2). *Spacing* is no choice of a
 * list: every screen follows the person's *Spacing* in *My settings › Appearance* (ADR-0034,
 * amendment). A timeline starts grouped by goal (UX §4.7); a board has no *Group by*: its rows come with its *Customize* (UX §4.6).
 */
export function ViewChoices({
  view,
  fields,
  onView,
  end,
  look = selectLook.box,
}: ViewChoicesProps) {
  const groupable = groupableOf(view.kind, fields);
  const groups = groupChoicesOf(fields, groupable);
  const grouped =
    view.groupBy !== null && Object.hasOwn(fields.names, view.groupBy) && groupable(view.groupBy);
  return (
    <>
      <SelectField
        label={t("timeline.show-as")}
        look={look}
        choices={shownAsChoices.map((choice) => ({ value: choice.value, label: t(choice.label) }))}
        value={view.kind}
        onChange={(kind: ShownAs) => {
          const starts = kind === "timeline" && view.kind !== "timeline";
          onView({ ...view, kind, groupBy: starts ? timelineStartGroup : view.groupBy });
        }}
      />
      {/* The map stands in columns by steps away and the rows of a board are chosen in its *Customize*: neither has groups. */}
      {view.kind === "map" || view.kind === "board" ? null : (
        <SelectField
          label={t("issues.group-by")}
          look={look}
          choices={[{ value: noGrouping, label: t("issues.group-by-none") }, ...groups]}
          value={grouped ? (view.groupBy ?? noGrouping) : noGrouping}
          onChange={(value) => {
            onView({ ...view, groupBy: value === noGrouping ? null : value });
          }}
        />
      )}
      {end}
    </>
  );
}
