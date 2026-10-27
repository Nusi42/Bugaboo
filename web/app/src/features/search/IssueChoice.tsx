import { t } from "@bugaboo/messages";
import { Button, buttonLook, PickerRow, SearchPicker, part, partOf } from "@bugaboo/ui";
import type { Operand, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { useAskedNames } from "./asked-names.ts";
import { nowhere } from "../../shared/search-tree.ts";

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

/**
 * Finding an issue by its Issue ID or by words of its title (M3-F62, UX design section 4.3). The
 * titles are found with the search route, so the person sees only issues they may read.
 */
export function IssueChoice({ language, values, onValues }: IssueChoiceProps) {
  const [words, setWords] = useState("");
  const chosen = chosenId(values);
  const found = useAskedNames(
    language,
    words.trim() === "" ? undefined : { what: "issues", startingWith: words.trim() },
  );
  return (
    <div {...partOf(part.searchList)}>
      {chosen === undefined ? null : (
        <p {...partOf(part.muted)}>{t("search.bar.issue-chosen", { id: chosen })}</p>
      )}
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
          <PickerRow key={issue.source} label={issue.label}>
            <Button
              look={buttonLook.quiet}
              aria-label={t("search.bar.issue-take", { issue: issue.label })}
              onPress={() => {
                onValues([
                  { type: "issue", id: { text: issue.source, span: nowhere }, span: nowhere },
                ]);
              }}
            >
              {t("search.bar.issue-choose")}
            </Button>
          </PickerRow>
        ))}
      </SearchPicker>
    </div>
  );
}
