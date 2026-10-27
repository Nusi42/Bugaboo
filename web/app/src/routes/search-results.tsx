import { lazy } from "react";

import type { ResultsInput } from "../shared/use-list-view.ts";

// The list under the search bar is the issue list, which is its own area and its own chunk.
const ResultsScreen = lazy(async () => ({
  default: (await import("../features/issues/index.ts")).SearchResults,
}));

/** The issue list under the search bar of a page that has no project. */
export function SearchResultsList(input: ResultsInput) {
  return <ResultsScreen {...input} />;
}
