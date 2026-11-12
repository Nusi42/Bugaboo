import { type Schema, dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { personName } from "../../shared/person.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys, refreshHistory } from "../../shared/query-keys.ts";
import { AS_ESTIMATED, factorText } from "../../shared/time-factor-text.ts";

/** The time factor of an issue as the server answers it for the reader (PLAN-14, ADR-0030). */
export type TimeFactor = Schema<"TimeFactorBody">;

/** How a new time factor is given: as a factor in percent, or as a length (*Takes me …*). */
export type NewTimeFactor = { readonly percent: number } | { readonly takes_minutes: number };

/**
 * The key of the time factor of an issue: among the times of the issue, so the page reads it again
 * once any change of the issue reached the server (a new first assigned person takes it back to ×1).
 */
function factorKey(issue: string) {
  return queryKeys.issues.time("factor", issue);
}

/** The time factor of an issue, read once for the page and again after each change of the issue. */
export function useTimeFactor(issue: string) {
  return useQuery({
    queryKey: factorKey(issue),
    queryFn: async (): Promise<TimeFactor> =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/time-factor", {
          params: { path: { issue } },
        }),
      ),
  });
}

/** Sets the time factor on the version of the issue that was looked at, and reads it again. */
export async function setTimeFactor(
  issue: string,
  version: number,
  factor: NewTimeFactor,
): Promise<void> {
  dataOf(
    await api.client.PUT("/api/v1/issues/{issue}/time-factor", {
      params: { path: { issue } },
      body: { version, ...factor },
    }),
  );
  // The issue counted up its version, and its history has the change.
  refreshHistory();
  await queryClient.invalidateQueries({ queryKey: queryKeys.issues.all });
}

/**
 * The factor the reader is shown: the field when their role sees it, and always their own estimate
 * when they are the person on the bar (the server leaves the masked members out of the answer).
 * None when the reader may not see it.
 */
export function shownPercent(factor: TimeFactor): number | undefined {
  if (factor.own_percent !== null) {
    return factor.own_percent;
  }
  return Object.hasOwn(factor, "percent") ? factor.percent : undefined;
}

/** Whether the reader is the person on the bar: only they are sent their own percent. */
export function isOwnFactor(factor: TimeFactor): boolean {
  return factor.own_percent !== null;
}

/** The row's words: "×1.25 · Lea's own estimate", "×1.5 · set by Ada Brandt" or "×1 (as estimated)". */
export function factorWords(factor: TimeFactor, percent: number): string {
  if (percent === AS_ESTIMATED) {
    return t("issues.time-factor-as-estimated");
  }
  const factorSaid = factorText(percent);
  // Who set it is part of the field: a person on the bar whose role does not see the field gets
  // their own factor without it.
  const setBy = Object.hasOwn(factor, "set_by") ? factor.set_by : null;
  if (factor.own_estimate && factor.person !== null) {
    return t("issues.time-factor-own", { factor: factorSaid, name: personName(factor.person) });
  }
  return setBy === null
    ? factorSaid
    : t("issues.time-factor-set-by", { factor: factorSaid, name: personName(setBy) });
}
