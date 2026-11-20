import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, DataCell, DataRow, DataRowHead, DataTable, partOf, part } from "@bugaboo/ui";

import { keptForText, kindName, type KeepRule } from "./keep-rules.ts";

/** Who deletes a kind of data after its keep period (`KeepGroup` of the API). */
type WhoDeletes = Schema<"KeepGroup">;

/** The groups in the order of the page: what Bugaboo deletes by itself, then what an administrator deletes. */
const GROUPS: readonly WhoDeletes[] = ["by-itself", "by-an-administrator"];

/** The names of the columns: on a phone they are said in the cells and the head is not shown. */
function columns() {
  return [
    t("keep-rules.col.kind"),
    t("keep-rules.col.period"),
    t("keep-rules.col.then"),
    t("keep-rules.col.why"),
  ];
}

/** One kind of data: its name, how long it is kept, what happens then, and why. */
function RuleRow({ rule }: { readonly rule: KeepRule }) {
  return (
    <DataRow>
      <DataRowHead>
        <span {...partOf(part.itemTitle)}>{kindName(rule.kind)}</span>
      </DataRowHead>
      <DataCell label={t("keep-rules.col.period")}>{keptForText(rule)}</DataCell>
      <DataCell label={t("keep-rules.col.then")}>{t(`keep-rules.then.${rule.group}`)}</DataCell>
      <DataCell label={t("keep-rules.col.why")}>
        <span {...partOf(part.muted)}>{t(`keep-kind.why.${rule.kind}`)}</span>
      </DataCell>
    </DataRow>
  );
}

/**
 * The keep and delete rules as two tables, one for each group with its sentence as the heading (UX §5.7, ADR-0045,
 * decision 2): every kind of data, how long it is kept, what happens then and why, in the order of the server's list.
 * *Keep and delete rules* and *My data* show them the same way.
 */
export function KeepRulesTables({ rules }: { readonly rules: readonly KeepRule[] }) {
  return GROUPS.map((group) => {
    const heading = t(`keep-group.name.${group}`);
    return (
      <Card key={group} heading={heading}>
        <DataTable label={heading} columns={columns()}>
          {rules
            .filter((rule) => rule.group === group)
            .map((rule) => (
              <RuleRow key={rule.kind} rule={rule} />
            ))}
        </DataTable>
      </Card>
    );
  });
}
