import { t } from "@bugaboo/messages";
import { DelayedSkeleton, useHoldWhileReading } from "@bugaboo/ui";

import { isDrawnColumn, withKeptColumns } from "../../shared/list-columns.ts";
import { useSearchFields, useSearchLanguageRead } from "../search/index.ts";
import { ChunkedView } from "./LazyViews.tsx";
import { LoadedResults } from "./LoadedResults.tsx";
import type { IssueResultsProps } from "./results-props.ts";

export type { IssueResultsProps } from "./results-props.ts";

/**
 * The results of a search as a list (UX section 4.2), for a project and for the *Search* page
 * alike: the columns a person chose, sorted by a click on a name, grouped by any field they may
 * read, and drawn only where the rows are on screen (PERF-04). *Show as › Timeline* draws them as
 * the timeline instead (UX §4.7), *Show as › Board* as columns of cards (UX §4.6) and *Show as › Map* as the
 * relation map of the results (UX §4.15) and *Show as › Calendar* as the calendar (UX §4.16).
 */
export function IssueResults(props: IssueResultsProps) {
  const languageRead = useSearchLanguageRead();
  const language = languageRead.data;
  const fields = useSearchFields(props.within);
  // The search bar says why the fields could not be read; the list has nothing to show then, and does not wait.
  // Inside the hold of the page of a search, the results wait for the fields with it.
  useHoldWhileReading(fields.isLoading || languageRead.isLoading);
  if (fields.isError) {
    return null;
  }
  if (language === undefined || fields.data === undefined) {
    return <DelayedSkeleton lines={3} />;
  }
  const { view, defaults } = props;
  if (
    view.kind === "timeline" ||
    view.kind === "board" ||
    view.kind === "map" ||
    view.kind === "calendar"
  ) {
    return <ChunkedView results={props} view={view} fields={fields.data} language={language} />;
  }
  const known = new Map(fields.data.fields.fields.map((field) => [field.word, field]));
  // Whatever the address or a saved search names, the columns that cannot be taken away are drawn.
  const drawn = (columns: readonly string[]): readonly string[] =>
    withKeptColumns(columns.filter((word) => isDrawnColumn(word, known.get(word))));
  return (
    <LoadedResults
      {...props}
      view={{ ...view, columns: drawn(view.columns) }}
      defaults={{ ...defaults, columns: drawn(defaults.columns) }}
      language={language}
      fields={{ ...fields.data, names: { id: t("issue-field.readable-id"), ...fields.data.names } }}
    />
  );
}
