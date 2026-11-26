import { DelayedSkeleton, HoldWhileLoading } from "@bugaboo/ui";
import type { SearchLanguage } from "@bugaboo/wasm-search";
import { lazy, Suspense, type ComponentProps, type ReactNode } from "react";

import type { ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import type { IssueResultsProps } from "./results-props.ts";

// The timeline is its own chunk, loaded when a search is first shown as one (coding-standards §6).
const TimelineResults = lazy(async () => ({
  default: (await import("../timeline/index.ts")).TimelineResults,
}));

// The board is its own chunk too, loaded when a search is first shown as one.
const BoardResults = lazy(async () => ({
  default: (await import("../board/index.ts")).BoardResults,
}));

// The calendar is its own chunk too, loaded when a search is first shown as one.
const CalendarResults = lazy(async () => ({
  default: (await import("../calendar/index.ts")).CalendarResults,
}));

// The relation map is its own chunk, loaded when a search is first shown as one (coding-standards §6).
const SearchMap = lazy(async () => ({
  default: (await import("../relation-map/index.ts")).SearchMap,
}));

/** What stands in the place of a view that is still being loaded: nothing, and the hold of the page waits for it. */
function WhileLoading({ children }: { readonly children: ReactNode }) {
  return (
    <Suspense
      fallback={
        <>
          <HoldWhileLoading />
          <DelayedSkeleton lines={3} />
        </>
      }
    >
      {children}
    </Suspense>
  );
}

/** The views that are chunks of their own, for the search the results are of. */
export interface ChunkedViewProps {
  readonly results: IssueResultsProps;
  readonly view: ListView;
  readonly fields: SearchFieldsAnswer;
  readonly language: SearchLanguage;
}

/** *Show as › Timeline* (UX §4.7), *Board* (UX §4.6), *Map* (UX §4.15) or *Calendar* (UX §4.16), whichever the view is. */
export function ChunkedView({ results, view, fields, language }: ChunkedViewProps) {
  const { text, within, onView } = results;
  if (view.kind === "map") {
    return (
      <WhileLoading>
        <SearchMap text={text} within={within} view={view} onView={onView} fields={fields} />
      </WhileLoading>
    );
  }
  if (view.kind === "calendar") {
    return (
      <WhileLoading>
        <CalendarResults
          text={text}
          within={within}
          view={view}
          onView={onView}
          fields={fields}
          language={language}
        />
      </WhileLoading>
    );
  }
  return view.kind === "board" ? (
    <BoardView
      text={text}
      onText={results.onText}
      within={within}
      view={view}
      onView={onView}
      keptIn={results.keptIn}
      fields={fields}
      language={language}
    />
  ) : (
    <TimelineView
      text={text}
      within={within}
      view={view}
      onView={onView}
      fields={fields}
      language={language}
    />
  );
}

/** *Show as › Timeline* (UX §4.7). */
export function TimelineView(props: ComponentProps<typeof TimelineResults>) {
  return (
    <WhileLoading>
      <TimelineResults {...props} />
    </WhileLoading>
  );
}

/** *Show as › Board* (UX §4.6). */
export function BoardView(props: ComponentProps<typeof BoardResults>) {
  return (
    <WhileLoading>
      <BoardResults {...props} />
    </WhileLoading>
  );
}
