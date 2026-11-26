import { sumWords, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { BarChart, NumberChart, type ChartRow } from "@bugaboo/ui";

import { formatNumber } from "../../shared/format.ts";
import { lengthSums } from "../../shared/length-sums.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { overText } from "../../shared/sum-text.ts";
import { groupName, groupsOfKeys, useOwnColumns } from "../issues/index.ts";
import { useSearchFields, useSearchLanguage } from "../search/index.ts";
import { chartShape, type Box } from "./dashboards.ts";

/** The totals of a search as a box gets them. */
type Totals = Schema<"SearchTotalsAnswer">;

/** The words of a chart of issues: the name of the box, the field it groups by, and the count. */
function labelsOf(box: Box, names: string) {
  return {
    name: box.title,
    names,
    values: t("dashboards.issues"),
    asTable: t("dashboards.as-table"),
  };
}

/**
 * A box of *Numbers and chart* (M10-Q13): bars of the issues of each group, in the order of the groups (a person field
 * in the order of the names; no sort by size, ADR-0050, decision 5), or one large number of all the issues; each with
 * its table.
 */
export function NumbersChart({ box, totals }: { readonly box: Box; readonly totals: Totals }) {
  const fields = useSearchFields();
  const by = box.by ?? undefined;
  // A group of an own field of numbers or days is written as the field writes its values, in the person's Region.
  const own = useOwnColumns(useSearchLanguage(), by !== undefined);
  if (box.shape === chartShape.number || by === undefined) {
    const rows: ChartRow[] = [
      {
        key: sumWords.count,
        name: t("dashboards.issues-found"),
        value: totals.all.issues,
        text: formatNumber(totals.all.issues),
      },
    ];
    return <NumberChart rows={rows} labels={labelsOf(box, t("dashboards.what"))} />;
  }
  const rows = groupsOfKeys(totals.groups, own[by], by).map((group): ChartRow => ({
    key: group.id,
    name: groupName(group),
    value: group.count,
    text: formatNumber(group.count),
  }));
  return (
    <BarChart
      rows={rows}
      labels={labelsOf(box, fields.data?.names[by] ?? by)}
      none={t("dashboards.no-issues")}
    />
  );
}

/** A box of *Time totals* of a search: *Estimated time*, *Time left* and *Time spent*, as numbers with their table. */
export function TimeNumbers({ box, totals }: { readonly box: Box; readonly totals: Totals }) {
  const rows = lengthSums.flatMap(({ sum: word, name }): ChartRow[] => {
    const sum = totals.all.sums[totals.sums.indexOf(word)];
    return sum === undefined
      ? []
      : [
          {
            key: word,
            name: t(name),
            value: typeof sum.value === "number" ? sum.value : 0,
            text: typeof sum.value === "number" ? minutesText(sum.value) : t("dashboards.no-value"),
            detail: typeof sum.value === "number" ? overText(sum, totals.all.issues) : undefined,
          },
        ];
  });
  return (
    <NumberChart
      rows={rows}
      labels={{
        name: box.title,
        names: t("dashboards.what"),
        values: t("dashboards.time"),
        asTable: t("dashboards.as-table"),
      }}
    />
  );
}
