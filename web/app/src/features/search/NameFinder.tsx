import { t } from "@bugaboo/messages";
import { FieldValue, PickerChoice, SearchPicker, type FieldLook } from "@bugaboo/ui";
import type { AskKind, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { useMe } from "../../shared/use-me.ts";
import { useAskedNames, type AskedName } from "./asked-names.ts";

export interface NameFinderProps {
  readonly language: SearchLanguage;
  /** What is found: people, groups, projects (the names the server lists for this person). */
  readonly what: Exclude<AskKind, "issues">;
  /** A name was chosen. */
  readonly onPick: (name: AskedName) => void;
  /** *Me* was chosen: for people, the first row of the list is *Me (Your Name)*. */
  readonly onPickMe?: () => void;
}

/** A person is shown with the round face of the avatar look (UX design section 7.5). */
const avatar: FieldLook = "avatar";

/** *Me (Your Name)*: the first row of the list of people. */
function MeChoice({ name, onPick }: { readonly name: string; readonly onPick: () => void }) {
  return (
    <PickerChoice
      label={
        <FieldValue look={avatar} text={t("search.bar.me-row", { name })} initialsFrom={name} />
      }
      onPick={onPick}
    />
  );
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
export function NameFinder({ language, what, onPick, onPickMe }: NameFinderProps) {
  const [words, setWords] = useState("");
  const asked = useAskedNames(language, { what, startingWith: words.trim() });
  const myName = useMe()?.name;
  // The person who searches is the first row, as *Me*, while the words typed are the start of their
  // name (or nothing is typed), and is not in the list a second time.
  const typed = words.trim().toLowerCase();
  const meRow =
    what === "people" &&
    onPickMe !== undefined &&
    myName !== undefined &&
    myName.toLowerCase().startsWith(typed);
  const found = meRow ? asked.filter((name) => name.value !== myName) : asked;
  return (
    <SearchPicker
      legend={legends[what]()}
      searchLabel={t("search.bar.pick-search")}
      clearLabel={t("search.bar.pick-clear")}
      listLabel={t("search.bar.pick-list")}
      query={words}
      onQuery={setWords}
      failed={undefined}
      nothing={found.length === 0 && !meRow ? t("search.bar.pick-none") : undefined}
      more={undefined}
    >
      {meRow ? <MeChoice name={myName} onPick={onPickMe} /> : null}
      {found.map((name) => (
        <PickerChoice
          key={name.source}
          label={what === "people" ? <FieldValue look={avatar} text={name.label} /> : name.label}
          {...(what === "people"
            ? {}
            : { accessibleName: t("search.bar.pick-take", { name: name.label }) })}
          onPick={() => {
            onPick(name);
          }}
        />
      ))}
    </SearchPicker>
  );
}
