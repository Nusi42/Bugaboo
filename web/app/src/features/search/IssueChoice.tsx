import { t } from "@bugaboo/messages";
import {
  PickerChoice,
  PickerId,
  SearchPicker,
  Skeleton,
  StatusPill,
  part,
  partOf,
} from "@bugaboo/ui";
import type { Operand, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { nowhere } from "../../shared/search-tree.ts";
import { useAskedNames } from "./asked-names.ts";
import { useSearchLanguage } from "./search-language.ts";

export interface IssueFinderProps {
  readonly language: SearchLanguage;
  /** An issue was chosen: its Issue ID as the language writes it. */
  readonly onPick: (id: string) => void;
}

/**
 * Finding an issue by its Issue ID or by words of its title (M3-F62, UX design section 4.3). The
 * titles are found with the search route, so the person sees only issues they may read.
 */
export function IssueFinder({ language, onPick }: IssueFinderProps) {
  const [words, setWords] = useState("");
  const found = useAskedNames(
    language,
    words.trim() === "" ? undefined : { what: "issues", startingWith: words.trim() },
  );
  return (
    <SearchPicker
      legend={t("search.bar.issue-legend")}
      searchLabel={t("search.bar.issue-search")}
      clearLabel={t("search.bar.issue-clear")}
      listLabel={t("search.bar.issue-list")}
      query={words}
      onQuery={setWords}
      failed={undefined}
      nothing={words.trim() !== "" && found.length === 0 ? t("search.bar.issue-none") : undefined}
      more={undefined}
    >
      {found.map((issue) => (
        <PickerChoice
          key={issue.source}
          label={
            issue.issue === undefined ? (
              issue.label
            ) : (
              <>
                <PickerId>{issue.issue.id}</PickerId>
                {issue.issue.title}
              </>
            )
          }
          accessibleName={t("search.bar.issue-take", { issue: issue.label })}
          onPick={() => {
            onPick(issue.source);
          }}
        >
          {issue.issue?.status === undefined ? null : (
            <StatusPill group={issue.issue.status.status_group} label={issue.issue.status.name} />
          )}
        </PickerChoice>
      ))}
    </SearchPicker>
  );
}

/** The issue finder for a screen that has no search bar (the *Issue* field): it fetches the language itself. */
export function IssuePicker({ onPick }: Pick<IssueFinderProps, "onPick">) {
  const language = useSearchLanguage();
  return language === undefined ? (
    <Skeleton lines={1} />
  ) : (
    <IssueFinder language={language} onPick={onPick} />
  );
}

export interface IssueChoiceProps {
  readonly language: SearchLanguage;
  /** The issue chosen so far, if any. */
  readonly values: readonly Operand[];
  readonly onValues: (values: readonly Operand[]) => void;
}

/** The ID in the text of an issue operand. */
function chosenId(values: readonly Operand[]): string | undefined {
  const [first] = values;
  return first?.type === "issue" ? first.id.text : undefined;
}

/** The value of a filter on an *Issue* field: the chosen issue, and the finder. */
export function IssueChoice({ language, values, onValues }: IssueChoiceProps) {
  const chosen = chosenId(values);
  return (
    <div {...partOf(part.searchList)}>
      {chosen === undefined ? null : (
        <p {...partOf(part.muted)}>{t("search.bar.issue-chosen", { id: chosen })}</p>
      )}
      <IssueFinder
        language={language}
        onPick={(id) => {
          onValues([{ type: "issue", id: { text: id, span: nowhere }, span: nowhere }]);
        }}
      />
    </div>
  );
}
