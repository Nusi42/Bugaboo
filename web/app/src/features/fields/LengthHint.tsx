import { t } from "@bugaboo/messages";
import { Length, SentenceWithValues } from "@bugaboo/ui";

import { formatNumber } from "../../shared/format.ts";

/** The hours that the example with a decimal means: an hour and a half. */
const EXAMPLE_HOURS = 1.5;

/**
 * What can be typed for a length: three ways to write ninety minutes, each drawn as a length in
 * the code font (UX §7.3), the words around them as the hint is.
 */
export function LengthHint() {
  const examples = [
    t("time.entry.length-example-colon"),
    formatNumber(EXAMPLE_HOURS),
    t("time.entry.length-example-short"),
  ];
  const [colon = "", decimal = "", short = ""] = examples;
  return (
    <SentenceWithValues
      sentence={t("time.entry.length-hint", { colon, decimal, short })}
      values={examples}
    >
      {(length) => <Length text={length} />}
    </SentenceWithValues>
  );
}
