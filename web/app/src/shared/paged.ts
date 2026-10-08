import { dataOf, pageSizeDefault, pageSizeMax, type Answer } from "@bugaboo/api-client";
import { useInfiniteQuery, type keepPreviousData, type QueryKey } from "@tanstack/react-query";
import { useEffect } from "react";

/**
 * How many items a request for a page of a list asks for. The most and the usual number are the
 * server's (`bb_core::paging`, generated); the short page is for a list to choose from, where a
 * few items and *Show more* are enough.
 */
export const pageSize = {
  /** The most a page holds: a list that is read whole in one page. */
  largest: pageSizeMax,
  /** What the server gives when the request says nothing. */
  usual: pageSizeDefault,
  /** A list to choose from, or a short list with *Show more* under it. */
  short: 20,
} as const;

/** The answer of a list: the items of the page, and where the next page starts, if there is one. */
interface CursorPage {
  readonly items: readonly unknown[];
  readonly next_cursor?: string | null | undefined;
}

/** The part of a request for a list that says which page is wanted: nothing for the first. */
export interface After {
  readonly cursor?: string;
}

/**
 * The names of fields of one item of a list, as the schema of the answer names them: a field that
 * does not exist cannot be listed.
 */
export type FieldNames<Item> = readonly Extract<keyof Item, string>[];

/**
 * The `fields` of a request for a list: the fields of one item that the screen shows and no more
 * (PERF-02). The names are checked against the item: `fieldList<Item>(["id", "name"])`.
 */
export function fieldList<Item extends object>(names: FieldNames<Item>): string {
  return names.join(",");
}

/** What a request for the pages of a list is made of. */
export interface PagedRequest<Page extends CursorPage> {
  /** The key of the list in the cache (`shared/query-keys.ts`). */
  readonly queryKey: QueryKey;
  /** Asks for a page: `after` is spread into the query of the request. */
  readonly read: (after: After) => Promise<Answer<Page>>;
}

/** The pages of a list, one at a time: `fetchNextPage` is *Show more*. */
export function usePaged<Page extends CursorPage>(
  request: PagedRequest<Page>,
  options: {
    readonly enabled?: boolean;
    readonly placeholderData?: typeof keepPreviousData;
  } = {},
) {
  return useInfiniteQuery({
    queryKey: request.queryKey,
    ...options,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) =>
      dataOf(await request.read(pageParam === null ? {} : { cursor: pageParam })),
    getNextPageParam: (last: Page) => last.next_cursor ?? null,
  });
}

/**
 * The lines of the pages read so far: each item becomes a line, or nothing when it came without
 * what a line needs (the fields of an item can be missing in an answer).
 */
export function linesFrom<Item, Line>(
  pages: readonly { readonly items: readonly Item[] }[] | undefined,
  lineOf: (item: Item) => Line | undefined,
): Line[] {
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => lineOf(item) ?? []));
}

/** What a query of pages needs to be read to its end. */
interface Pageable {
  readonly hasNextPage: boolean;
  readonly isFetchingNextPage: boolean;
  readonly fetchNextPage: () => Promise<unknown>;
}

/**
 * Reads the next page as soon as the one before has arrived, until there is none: for a list that
 * is a choice in a menu (the roles), which must offer every one of them and not the first page.
 */
export function useAllPages(query: Pageable): void {
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query;
  useEffect(() => {
    if (hasNextPage && !isFetchingNextPage) {
      void fetchNextPage();
    }
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);
}
