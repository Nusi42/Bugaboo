import { t } from "@bugaboo/messages";
import { DelayedSkeleton, HoldWhileLoading, useHoldWhileReading } from "@bugaboo/ui";
import { lazy, Suspense } from "react";

import { isDrawnColumn, withKeptColumns } from "../../shared/list-columns.ts";
import type { SearchLanguage } from "@bugaboo/wasm-search";

import {
  useSearchFields,
  useSearchLanguageRead,
  type SearchFieldsAnswer,
} from "../search/index.ts";
import { LoadedResults } from "./LoadedResults.tsx";
import type { IssueResultsProps } from "./results-props.ts";

export type { IssueResultsProps } from "./results-props.ts";

// The timeline is its own chunk, loaded when a search is first shown as one (coding-standards §6).
const TimelineResults = lazy(async () => ({
  default: (await import("../timeline/index.ts")).TimelineResults,
}));

// The relation map is its own chunk, loaded when a search is first shown as one (coding-standards §6).
const SearchMap = lazy(async () => ({
  default: (await import("../relation-map/index.ts")).SearchMap,
}));

/** What stands in the place of the results while a view of its own chunk loads. */
function Loading() {
  return (
    <>
      <HoldWhileLoading />
      <DelayedSkeleton lines={3} />
    </>
  );
}

/** A view that is its own chunk: the timeline (UX §4.7) or the relation map (UX §4.15) of the results. */
function OwnChunkView({
  text,
  within,
  view,
  onView,
  fields,
  language,
}: Pick<IssueResultsProps, "text" | "within" | "view" | "onView"> & {
  readonly fields: SearchFieldsAnswer;
  readonly language: SearchLanguage;
}) {
  return view.kind === "map" ? (
    <SearchMap text={text} within={within} view={view} onView={onView} fields={fields} />
  ) : (
    <TimelineResults
      text={text}
      within={within}
      view={view}
      onView={onView}
      fields={fields}
      language={language}
    />
  );
}

/**
 * The results of a search as a list (UX section 4.2), for a project and for the *Search* page
 * alike: the columns a person chose, sorted by a click on a name, grouped by any field they may
 * read, and drawn only where the rows are on screen (PERF-04). *Show as › Timeline* draws them as
 * the timeline instead (UX §4.7), and *Show as › Map* as the relation map of the results (UX §4.15).
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
  if (view.kind === "map" || view.kind === "timeline") {
    return (
      <Suspense fallback={<Loading />}>
        <OwnChunkView {...props} fields={fields.data} language={language} />
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
