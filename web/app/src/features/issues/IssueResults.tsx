import { t } from "@bugaboo/messages";
import { Skeleton } from "@bugaboo/ui";

import { isDrawnColumn } from "../../shared/list-columns.ts";
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
  const known = new Map(fields.data.fields.fields.map((field) => [field.word, field]));
  const drawn = (columns: readonly string[]): readonly string[] =>
    columns.filter((word) => isDrawnColumn(word, known.get(word)));
  return (
    <LoadedResults
      {...props}
      view={{ ...props.view, columns: drawn(props.view.columns) }}
      defaults={{ ...props.defaults, columns: drawn(props.defaults.columns) }}
      language={language}
      fields={{ ...fields.data, names: { id: t("issue-field.readable-id"), ...fields.data.names } }}
    />
  );
}
