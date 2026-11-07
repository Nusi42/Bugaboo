import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** How a text that AI wrote was written (COMP-08, COMP-09); a text written by hand has none. */
export type Mark = Schema<"WrittenByBody">;

/**
 * The words shown beside a text that AI wrote: *Written with the assistant*, *Written by an AI app*
 * or *Written by an agent*, with the name of the app or of the agent when the server gave it.
 */
export function writtenByText(mark: Mark): string {
  if (mark.kind === "assistant") {
    return t("written-by.assistant");
  }
  if (mark.kind === "app") {
    const app = mark.app ?? "";
    return app === "" ? t("written-by.app-unnamed") : t("written-by.app", { app });
  }
  const agent = mark.agent ?? "";
  return agent === "" ? t("written-by.agent-unnamed") : t("written-by.agent", { agent });
}
