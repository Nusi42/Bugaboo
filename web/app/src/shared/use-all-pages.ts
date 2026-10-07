import { useEffect } from "react";

/** What a query of pages needs to be read to its end. */
interface Paged {
  readonly hasNextPage: boolean;
  readonly isFetchingNextPage: boolean;
  readonly fetchNextPage: () => Promise<unknown>;
}

/**
 * Reads the next page as soon as the one before has arrived, until there is none: for a list that
 * is a choice in a menu (the roles), which must offer every one of them and not the first page.
 */
export function useAllPages(query: Paged): void {
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = query;
  useEffect(() => {
    if (hasNextPage && !isFetchingNextPage) {
      void fetchNextPage();
    }
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);
}
