import { dataOf } from "@bugaboo/api-client";

import { codeOf, type Context } from "./context.ts";
import { HIDDEN_FIELD_WORD } from "./organization.ts";
import { pick } from "./random.ts";

// Searching while the others change data (architecture §11, "Searches while others change data"):
// every person searches now and then, and writes down what they were told. `bb-load-check` judges
// it: no issue the person may not read, a search for everything finds exactly what they read, the
// count line agrees, and the own field hidden from Guests is refused for a Guest and answered for
// everybody else.

/** How many issues a page holds: more than the run has. */
const PAGE = 40;

/** What a search asks for: its text and what the judge may hold it to. */
interface Asking {
  readonly q: string;
  /** Every issue the person reads, which no change of the run alters. */
  readonly everything: boolean;
  /** The search reads the own field that Guests may not see. */
  readonly readsHidden: boolean;
}

/** The searches people make: the same list for everybody, the judge knows what each means. */
const askings: readonly Asking[] = [
  { q: "", everything: true, readsHidden: false },
  { q: "project is LOAD sort by created", everything: true, readsHidden: false },
  { q: "statusGroup is not Done", everything: false, readsHidden: false },
  { q: 'contains "issue"', everything: false, readsHidden: false },
  { q: `${HIDDEN_FIELD_WORD} is "x"`, everything: false, readsHidden: true },
];

/** One search, with the count line for a search for everything; what the person was told is written. */
export async function searchNow(context: Context): Promise<void> {
  const asking = askings[pick(context.random, askings.length)];
  if (asking === undefined) {
    return;
  }
  const note = {
    holder: context.holder,
    query: asking.q,
    everything: asking.everything,
    reads_hidden: asking.readsHidden,
  };
  try {
    const page = dataOf(
      await context.client.GET("/api/v1/search/issues", {
        params: { query: { q: asking.q, limit: PAGE } },
      }),
    );
    const found: string[] = [];
    for (const item of page.items) {
      found.push(String(item.readable_id));
    }
    const more = page.next_cursor !== null && page.next_cursor !== undefined;
    if (!asking.everything) {
      context.book.searched({ ...note, found, more });
      return;
    }
    const count = dataOf(
      await context.client.GET("/api/v1/search/issues/count", {
        params: { query: { q: asking.q } },
      }),
    );
    context.book.searched({ ...note, found, more, count: count.issues });
  } catch (error) {
    const code = codeOf(error);
    if (code === undefined) {
      throw error;
    }
    context.book.searched({ ...note, found: [], more: false, refused: code });
  }
}
