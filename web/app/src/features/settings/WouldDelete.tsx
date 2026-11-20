import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { KeptRoom, partOf, part } from "@bugaboo/ui";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { recordsText, type DataKind } from "./keep-rules.ts";

/** What a changed period would delete, or leave past it, as the server counts it now. */
function wouldDeleteOptions(kind: DataKind, count: number) {
  return api.query.queryOptions("get", "/api/v1/admin/keep-rules/{kind}/would-delete", {
    params: { path: { kind }, query: { count } },
  });
}

/** What a period would do, in words: what Bugaboo deletes within a day, or what is past it for an administrator. */
function effectText(effect: Schema<"PeriodEffectBody">): string {
  const records = recordsText(effect.kind, effect.records);
  if (effect.group === "by-itself") {
    return effect.records === 0
      ? t("keep-rules.would-delete-none")
      : t("keep-rules.would-delete", { records });
  }
  return effect.records === 0
    ? t("keep-rules.would-be-past-none")
    : t("keep-rules.would-be-past", { records, count: effect.records });
}

/**
 * What the period typed would delete, before it is saved: "With this period, Bugaboo deletes 312 notifications within a
 * day." The line keeps its room while the count is read, and shows the last count until the next is there, so nothing
 * under it moves while the person types (UX §6.4).
 */
export function WouldDelete({ kind, count }: { readonly kind: DataKind; readonly count: number }) {
  const effect = useQuery({
    ...wouldDeleteOptions(kind, count),
    placeholderData: keepPreviousData,
  });
  return (
    <p {...partOf(part.muted)}>
      {effect.data === undefined ? (
        <KeptRoom>{t("keep-rules.would-delete-none")}</KeptRoom>
      ) : (
        effectText(effect.data)
      )}
    </p>
  );
}
