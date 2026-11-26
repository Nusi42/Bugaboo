import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** Over how many of the issues a sum is, where fewer gave a value than the search found ("over 9 of 14 issues"). */
export function overText(sum: Schema<"SearchSumBody">, issues: number): string | undefined {
  return sum.valued < issues
    ? t("issues.totals-over-part", { valued: sum.valued, issues })
    : undefined;
}

/**
 * A sum of a total as people read it (QRY-08, M10-Q15): its value in words, and where fewer issues gave a value than the
 * search found, over how many of them ("12h over 9 of 14 issues"); none where no issue gave a value the person may
 * read. The list's totals row and the boxes of a dashboard say it the same way.
 */
export function sumText(
  sum: Schema<"SearchSumBody">,
  issues: number,
  valueText: (value: unknown) => string,
): string | undefined {
  if (sum.value === null || sum.value === undefined) {
    return undefined;
  }
  const total = valueText(sum.value);
  return sum.valued < issues
    ? t("issues.totals-over", { total, valued: sum.valued, issues })
    : total;
}
