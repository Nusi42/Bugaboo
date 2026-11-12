import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { wordedChange } from "../../shared/change-sentence.ts";
import { renderSaid } from "../../shared/said.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { RunMark } from "./RunMark.tsx";
import { WordedItem } from "./WordedItem.tsx";

/**
 * The lines of the field changes of one event. The shared wording says them (`bb-core`, the same
 * rule that words a notification): a sentence for each change, unless it says some of them together
 * (both dates moving in one event). Then its sentences are the lines; otherwise each change is its
 * own line, with its lengths set apart.
 */
export function ChangeLines({ items }: { readonly items: readonly Schema<"HistoryEntryItem">[] }) {
  return (
    <WithWording>
      {(wording) => {
        const said = wording.changeSentences(items.map((item) => wordedChange(item)));
        const first = items[0];
        if (said.length === items.length || first === undefined) {
          return items.map((item) => (
            <li key={item.change?.field ?? item.event} {...partOf(part.itemText)}>
              <WordedItem item={item} />
              <RunMark item={item} />
            </li>
          ));
        }
        return said.map((sentence) => (
          <li key={renderSaid(sentence)} {...partOf(part.itemText)}>
            <span>{renderSaid(sentence)}</span>
            <RunMark item={first} />
          </li>
        ));
      }}
    </WithWording>
  );
}
