import catalog from "../../../../messages/en.json" with { type: "json" };

/**
 * The words the server writes into the data of a new organization: the names of the starter
 * workflows and the texts of their buttons. The browser shows them as the API sends them and never
 * loads them, so a test reads them from the catalog the server reads (`messages/en.json`). The
 * keys are the ones of the catalog that start so, never a list written a second time.
 */
export type ServerWord = Extract<
  keyof typeof catalog,
  `workflow.${string}` | `workflow-change.${string}`
>;

/** The text of a word the server writes, in the language of the catalog. */
export function serverWord(key: ServerWord): string {
  return catalog[key];
}
