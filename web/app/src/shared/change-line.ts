import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import type { Wording } from "@bugaboo/wasm";

import { wordedSideOf } from "./change-sentence.ts";
import { fieldNameOf } from "./field-names.ts";
import { renderSaid } from "./said.ts";

/** What a field is on one side of a change, in words; *nothing* for no value. */
function sideText(
  wording: Wording,
  change: Schema<"FieldChangeBody">,
  value: Schema<"FieldChangeBody">["after"],
): string {
  const side = wordedSideOf(value);
  const said = side === null ? null : wording.sideWords(change.field, side);
  return said === null ? t("common.nothing") : renderSaid(said);
}

/**
 * The line that says what one field of an issue is after a change: *Priority: High*. The preview of
 * a change to many issues and the changes of a run say it the same way.
 */
export function changeLineText(wording: Wording, change: Schema<"FieldChangeBody">): string {
  return t("preview.line-changes", {
    field: fieldNameOf({ code: change.field, name: change.name }),
    after: sideText(wording, change, change.after),
  });
}

/** The line that says what one field was and is: *Priority: Normal → Critical*. */
export function changeArrowText(wording: Wording, change: Schema<"FieldChangeBody">): string {
  return t("runs.change-line", {
    field: fieldNameOf({ code: change.field, name: change.name }),
    before: sideText(wording, change, change.before),
    after: sideText(wording, change, change.after),
  });
}
