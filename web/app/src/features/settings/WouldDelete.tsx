import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { KeptRoom, partOf, part } from "@bugaboo/ui";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
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

/** The words of the line: the hint for the stored period, the count, the refusal of the server, or none yet. */
function lineText(
  effect: { readonly data?: Schema<"PeriodEffectBody"> | undefined; readonly error: unknown },
  count: number | undefined,
): string | undefined {
  if (count === undefined) {
    return t("keep-rules.would-delete-hint");
  }
  if (effect.error !== null) {
    return messageOf(effect.error);
  }
  return effect.data === undefined ? undefined : effectText(effect.data);
}

/**
 * What the period typed would delete, before it is saved: "With this period, Bugaboo deletes 312 notifications within a
 * day." It is counted only for a changed period (`count`); under the stored period the line says how to see it, so the
 * box has no empty line and the line is there from the start. While the first count is read the line keeps its room,
 * and it shows the last count until the next is there, so nothing under it moves while the person types (UX §6.4). A
 * count the server refuses is said in its words.
 */
export function WouldDelete({
  kind,
  count,
}: {
  readonly kind: DataKind;
  readonly count: number | undefined;
}) {
  const effect = useQuery({
    ...wouldDeleteOptions(kind, count ?? 0),
    enabled: count !== undefined,
    placeholderData: keepPreviousData,
  });
  const text = lineText(effect, count);
  return (
    <p {...partOf(part.muted)}>
      {text ?? <KeptRoom>{t("keep-rules.would-delete-none")}</KeptRoom>}
    </p>
  );
}
