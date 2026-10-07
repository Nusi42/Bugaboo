import { t } from "@bugaboo/messages";
import { Alert, Button, SearchField } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { recentFirst } from "../../shared/recent-choices.ts";
import { type Choice, choicesOf, useMemberChoices, type GroupLine } from "./groups.ts";
import styles from "./People.module.css";
import type { MemberChanges } from "./use-member-changes.ts";

export interface MemberPickerProps {
  readonly group: GroupLine;
  readonly changes: MemberChanges;
}

/** The people found, each with a button that puts them in, or a word that they are in already. */
function Options({
  choices,
  changes,
}: {
  readonly choices: readonly Choice[];
  readonly changes: MemberChanges;
}) {
  return (
    <ul className={styles.options} aria-label={t("groups.add-list")}>
      {choices.map((person) => (
        <li key={person.id} className={styles.option}>
          <span className={styles.optionName}>{person.name}</span>
          {person.alreadyIn ? (
            <span className={styles.optionHint}>{t("groups.in-already")}</span>
          ) : (
            <Button
              aria-label={t("groups.add-name", { name: person.name })}
              onPress={() => {
                changes.add(person);
              }}
            >
              {t("groups.add")}
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}

/**
 * Putting people into the group (UX §6): a search box, and the people it finds by name with a button
 * each; those who are in already say so, and the people picked last come first.
 */
export function MemberPicker({ group, changes }: MemberPickerProps) {
  const [query, setQuery] = useState("");
  const found = useMemberChoices(group.id, query.trim());
  const choices = choicesOf(found.data?.pages);
  const shown = query.trim() === "" ? recentFirst("person", choices) : choices;
  return (
    <fieldset className={styles.picker}>
      <legend className={styles.legend}>{t("groups.add-title")}</legend>
      <SearchField
        label={t("groups.add-search")}
        clearLabel={t("search.clear")}
        value={query}
        onChange={setQuery}
      />
      <Options choices={shown} changes={changes} />
      {found.isError ? <Alert danger>{messageOf(found.error)}</Alert> : null}
      {found.isSuccess && shown.length === 0 ? (
        <p className={styles.muted}>{t("groups.add-nothing")}</p>
      ) : null}
      {found.hasNextPage ? (
        <Button
          quiet
          isPending={found.isFetchingNextPage}
          onPress={() => {
            void found.fetchNextPage();
          }}
        >
          {t("groups.add-more")}
        </Button>
      ) : null}
    </fieldset>
  );
}
