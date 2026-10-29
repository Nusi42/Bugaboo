import type { ResultsInput } from "../../shared/use-list-view.ts";
import { IssueResults } from "./IssueResults.tsx";
import { useOpenIssuesIn } from "./open-in.ts";
import { SearchToolbar } from "./ProjectToolbar.tsx";
import { useSelection } from "./use-selection.ts";

/**
 * The list under the search bar of the *Search* page and of a saved search (UX section 4.2): the
 * issues of every project the person sees, with the *Project* column. The changes of many issues
 * are made inside one project, so these rows have no box to tick.
 */
export function SearchResults(input: ResultsInput) {
  return (
    <>
      <SearchToolbar />
      <IssueResults
        {...input}
        within={undefined}
        scopeName={undefined}
        project={undefined}
        openIn={useOpenIssuesIn()}
        state={undefined}
        open={undefined}
        selection={useSelection()}
      />
    </>
  );
}
