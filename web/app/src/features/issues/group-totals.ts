import { dataOf, sumWords, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { listWord } from "../../shared/list-columns.ts";
import { ownValueText } from "../../shared/own-value-text.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { sumText } from "../../shared/sum-text.ts";
import type { FoundSearch } from "./found-issues.ts";
import type { OwnColumns } from "./own-columns.ts";

/** The sum of each column of a length, by the search word of its column. */
const lengthSums: Readonly<Record<string, string>> = {
  [listWord.estimatedTime]: sumWords.estimated,
  [listWord.timeLeft]: sumWords.timeLeft,
  [listWord.timeSpent]: sumWords.timeSpent,
};

/** The kinds of own field that a total adds up. */
const numberKinds: ReadonlySet<Schema<"FieldType">> = new Set(["whole-number", "decimal"]);

/** The sum a column of the list asks for: a length, or an own field of numbers; none for another column. */
export function sumOf(word: string, own: OwnColumns): string | undefined {
  const length = lengthSums[word];
  if (length !== undefined) {
    return length;
  }
  const field = own[word];
  return field !== undefined && numberKinds.has(field.kind)
    ? `${sumWords.number}${word}`
    : undefined;
}

/** The sums of the columns of the list, in the order of the columns. */
export function sumsOf(words: readonly string[], own: OwnColumns): readonly string[] {
  return words.flatMap((word) => sumOf(word, own) ?? []);
}

/**
 * The totals of a grouped list (QRY-08, M10-Q15): one read for every group of the list, by the field it is grouped by,
 * with the sums of its columns of lengths and own numbers; none while it is not grouped.
 */
export function useGroupTotals(search: FoundSearch, by: string | null, sums: readonly string[]) {
  return useQuery({
    queryKey: queryKeys.search.totals(search.text, search.within, by ?? "", sums.join(",")),
    placeholderData: keepPreviousData,
    enabled: by !== null,
    queryFn: async () =>
      dataOf(
        await api.client.POST("/api/v1/search/totals", {
          body: {
            ...(search.text === "" ? {} : { q: search.text }),
            ...(search.within === undefined ? {} : { within: search.within }),
            by,
            sums: [...sums],
          },
        }),
      ),
  });
}

/** The words of the sum of one column of a row of totals; none for a column without a sum or a value. */
export function columnTotal(
  word: string,
  row: Schema<"SearchTotalRowBody">,
  answer: Schema<"SearchTotalsAnswer">,
  own: OwnColumns,
): string | undefined {
  const sum = sumOf(word, own);
  const value = sum === undefined ? undefined : row.sums[answer.sums.indexOf(sum)];
  if (value === undefined) {
    return undefined;
  }
  const field = own[word];
  return sumText(value, row.issues, (total) =>
    field === undefined
      ? minutesText(typeof total === "number" ? total : 0)
      : (ownValueText(field.kind, total) ?? ""),
  );
}
