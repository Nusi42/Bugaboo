import { Skeleton } from "@bugaboo/ui";

import { useSearchFields, useSearchLanguage } from "../search/index.ts";
import { LoadedResults } from "./LoadedResults.tsx";
import type { IssueResultsProps } from "./results-props.ts";

export type { IssueResultsProps } from "./results-props.ts";

/**
 * The results of a search as a list (UX section 4.2), for a project and for the *Search* page
 * alike: the columns a person chose, sorted by a click on a name, grouped by any field they may
 * read, and drawn only where the rows are on screen (PERF-04).
 */
export function IssueResults(props: IssueResultsProps) {
  const language = useSearchLanguage();
  const fields = useSearchFields(props.within);
  if (language === undefined || fields.data === undefined) {
    return <Skeleton lines={3} />;
  }
  return <LoadedResults {...props} language={language} fields={fields.data} />;
}
