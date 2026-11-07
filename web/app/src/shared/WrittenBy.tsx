import { partOf, part } from "@bugaboo/ui";

import { writtenByText, type Mark } from "./written-by.ts";

/** The mark beside a text that AI wrote, in the quiet style of the time of a comment. */
export function WrittenBy({ mark }: { readonly mark: Mark | null | undefined }) {
  if (mark === null || mark === undefined) {
    return null;
  }
  return <span {...partOf(part.writtenBy)}>{writtenByText(mark)}</span>;
}
