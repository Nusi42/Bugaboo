import { t } from "@bugaboo/messages";
import { PickerChoice, SearchPicker } from "@bugaboo/ui";
import type { AskKind, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { useAskedNames, type AskedName } from "./asked-names.ts";

export interface NameFinderProps {
  readonly language: SearchLanguage;
  /** What is found: people, groups, projects (the names the server lists for this person). */
  readonly what: Exclude<AskKind, "issues">;
  /** A name was chosen. */
  readonly onPick: (name: AskedName) => void;
}

/** The words of the finder for each kind of name. */
const legends: Readonly<Record<NameFinderProps["what"], () => string>> = {
  people: () => t("search.bar.pick-people"),
  groups: () => t("search.bar.pick-groups"),
  projects: () => t("search.bar.pick-projects"),
};

/**
 * Finding a person, a group or a project by the start of the name (UX design section 4.3): the
 * names come from the routes of the search, which list only those this person may use in a search.
 */
export function NameFinder({ language, what, onPick }: NameFinderProps) {
  const [words, setWords] = useState("");
  const found = useAskedNames(language, { what, startingWith: words.trim() });
  return (
    <SearchPicker
      legend={legends[what]()}
      searchLabel={t("search.bar.pick-search")}
      clearLabel={t("search.bar.pick-clear")}
      listLabel={t("search.bar.pick-list")}
      query={words}
      onQuery={setWords}
      failed={undefined}
      nothing={found.length === 0 ? t("search.bar.pick-none") : undefined}
      more={undefined}
    >
      {found.map((name) => (
        <PickerChoice
          key={name.source}
          label={name.label}
          accessibleName={t("search.bar.pick-take", { name: name.label })}
          onPick={() => {
            onPick(name);
          }}
        />
      ))}
    </SearchPicker>
  );
}
