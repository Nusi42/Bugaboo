import { t } from "@bugaboo/messages";
import { Skeleton } from "@bugaboo/ui";
import { lazy, Suspense } from "react";

import { isDrawnColumn, withKeptColumns } from "../../shared/list-columns.ts";
import { useSearchFields, useSearchLanguage } from "../search/index.ts";
import { LoadedResults } from "./LoadedResults.tsx";
import type { IssueResultsProps } from "./results-props.ts";

export type { IssueResultsProps } from "./results-props.ts";

// The timeline is its own chunk, loaded when a search is first shown as one (coding-standards §6).
const TimelineResults = lazy(async () => ({
  default: (await import("../timeline/index.ts")).TimelineResults,
}));

/**
 * The results of a search as a list (UX section 4.2), for a project and for the *Search* page
 * alike: the columns a person chose, sorted by a click on a name, grouped by any field they may
 * read, and drawn only where the rows are on screen (PERF-04). *Show as › Timeline* draws them as
 * the timeline instead (UX §4.7).
 */
export function IssueResults(props: IssueResultsProps) {
  const language = useSearchLanguage();
  const fields = useSearchFields(props.within);
  if (language === undefined || fields.data === undefined) {
    return <Skeleton lines={3} />;
  }
  const { view, defaults } = props;
  if (view.kind === "timeline") {
    return (
      <Suspense fallback={<Skeleton lines={3} />}>
        <TimelineResults
          text={props.text}
          within={props.within}
          view={view}
          onView={props.onView}
          fields={fields.data}
          language={language}
        />
      </Suspense>
    );
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
