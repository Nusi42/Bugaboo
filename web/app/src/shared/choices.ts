import type { Schema } from "@bugaboo/api-client";

import { linesFrom } from "./paged.ts";

/** One thing to choose by name from a page of choices of the server: a person, a group or a role. */
export interface Choice {
  readonly id: string;
  readonly name: string;
}

/** The choices in the pages read so far; one the server left without an ID or a name is not offered. */
export function choicesOf(pages: readonly Schema<"Page_ChoiceItem">[] | undefined): Choice[] {
  return linesFrom(pages, (item) =>
    item.id === undefined || item.id === null || item.name === undefined || item.name === null
      ? undefined
      : { id: item.id, name: item.name },
  );
}
