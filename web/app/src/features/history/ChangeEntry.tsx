import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { changeSentence, relationSentence } from "./history.ts";

/** What tells the items of one event apart: the field that changed, or the relation. */
function keyOf(item: Schema<"HistoryEntryItem">): string {
  return [item.event, item.field ?? item.relation ?? ""].join(" ");
}

/**
 * The changes of one event as one entry: a sentence for each field that changed, and when. A
 * relation that was added or taken away is a sentence too. What the reader may not see is not in
 * the entries at all (SEC-06).
 */
export function ChangeEntry({ items }: { readonly items: readonly Schema<"HistoryEntryItem">[] }) {
  const first = items[0];
  if (first === undefined) {
    return null;
  }
  return (
    <article {...partOf(part.historyEntry)}>
      <span {...partOf(part.historyWhen)}>{formatDateTime(first.at)}</span>
      <ul {...partOf(part.list)}>
        {items.map((item) => (
          <li key={keyOf(item)} {...partOf(part.itemText)}>
            {item.kind === "relation" ? relationSentence(item) : changeSentence(item)}
          </li>
        ))}
      </ul>
    </article>
  );
}

/** What the history says when there is nothing in it yet. */
export function NothingYet() {
  return <p {...partOf(part.muted)}>{t("history.nothing")}</p>;
}
