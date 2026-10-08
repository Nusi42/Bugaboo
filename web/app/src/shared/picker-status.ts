import type { SearchPickerProps } from "@bugaboo/ui";

import { messageOf } from "./errors.ts";

/** What a picker needs to know about the list it reads a page at a time. */
interface PagedChoices {
  readonly error: unknown;
  readonly isSuccess: boolean;
  readonly hasNextPage: boolean;
  readonly isFetchingNextPage: boolean;
  readonly fetchNextPage: () => Promise<unknown>;
}

/** The words a picker says when nothing was found and on the button for the next page. */
export interface PickerTexts {
  readonly nothing: string;
  readonly more: string;
}

/**
 * The parts of a picker that follow from its list: the failure in plain words, the word that
 * nothing was found (when the list was read and `shown` is none), and *Show more* while there is
 * a next page.
 */
export function pickerStatus(
  found: PagedChoices,
  shown: number,
  texts: PickerTexts,
): Pick<SearchPickerProps, "failed" | "nothing" | "more"> {
  return {
    failed: found.error === null || found.error === undefined ? undefined : messageOf(found.error),
    nothing: found.isSuccess && shown === 0 ? texts.nothing : undefined,
    more: found.hasNextPage
      ? {
          label: texts.more,
          pending: found.isFetchingNextPage,
          press: () => {
            void found.fetchNextPage();
          },
        }
      : undefined,
  };
}
