import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";

/** A kind of data that is deleted after its keep period (`KeepKind` of the API). */
export type DataKind = Schema<"KeepKind">;

/** Work days, the kind an administrator deletes that *Working hours* shows too. */
export const workDaysKind: DataKind = "work-days";

/** One row of *Keep and delete rules*, as the server sends it. */
export type KeepRule = Schema<"KeepRuleBody">;

/** The rules with the periods in force: every person who signs in may read them (ADR-0045, amendment M9). */
export function keepRulesOptions() {
  return api.query.queryOptions("get", "/api/v1/admin/keep-rules");
}

/** The name of a kind of data: *Notifications*. */
export function kindName(kind: DataKind): string {
  return t(`keep-kind.name.${kind}`);
}

/** Some records of a kind, in words: *312 notifications*. */
export function recordsText(kind: DataKind, count: number): string {
  return t(`keep-kind.count.${kind}`, { count });
}

/** A keep period in words: *90 days*. */
export function periodText(period: Schema<"LengthBody">): string {
  return t(`keep-length.${period.unit}`, { count: period.count });
}

/**
 * How long a kind is kept, in words: its period, or, for the rotation of backups, that no day can be named and how far
 * back they reach (`fixed_end`, which the server decides).
 */
export function keptForText(rule: KeepRule): string {
  const period = periodText(rule.period);
  return rule.fixed_end ? period : t("keep-rules.no-fixed-end", { length: period });
}
