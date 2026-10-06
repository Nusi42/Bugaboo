import { t } from "@bugaboo/messages";
import { Alert, Button, SearchField } from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { recentFirst } from "../../shared/recent-choices.ts";
import { choicesOf, useChoices, type Choice } from "./assignments.ts";
import styles from "./Roles.module.css";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ChoicePickerProps {
  /** Whom to pick: a person, or a group. */
  readonly kind: "person" | "group";
  /** The project the row is for (an ID), or `null` for the whole organization. */
  readonly project: string | null;
  /** The ID of the one picked, if any. */
  readonly value: string | null;
  readonly onPick: (choice: Choice) => void;
}

/** The people or groups found, as radio buttons. */
function Options({
  choices,
  value,
  onPick,
}: {
  readonly choices: readonly Choice[];
  readonly value: string | null;
  readonly onPick: (choice: Choice) => void;
}) {
  const group = useId();
  return (
    <ul className={styles.options} aria-label={t("assignments.pick-list")}>
      {choices.map((choice) => (
        <li key={choice.id}>
          <label className={styles.option}>
            <input
              type={radio}
              name={group}
              checked={value === choice.id}
              onChange={() => {
                onPick(choice);
              }}
            />
            <span className={styles.optionName}>{choice.name}</span>
          </label>
        </li>
      ))}
    </ul>
  );
}

/**
 * Whom to give a role to (UX §6): a search box and the people or groups it finds by name, as radio
 * buttons; the ones picked last come first. Names only: the server never offers an e-mail address.
 */
export function ChoicePicker({ kind, project, value, onPick }: ChoicePickerProps) {
  const [query, setQuery] = useState("");
  const found = useChoices(kind, project, query.trim());
  const all = choicesOf(found.data?.pages);
  const shown = query.trim() === "" ? recentFirst(kind, all) : all;
  const people = kind === "person";
  return (
    <fieldset className={styles.picker}>
      <legend className={styles.legend}>
        {t(people ? "assignments.pick-person" : "assignments.pick-group")}
      </legend>
      <SearchField
        label={t(people ? "assignments.search-people" : "assignments.search-groups")}
        clearLabel={t("search.clear")}
        value={query}
        onChange={setQuery}
      />
      <Options choices={shown} value={value} onPick={onPick} />
      {found.isError ? <Alert danger>{messageOf(found.error)}</Alert> : null}
      {found.isSuccess && shown.length === 0 ? (
        <p className={styles.muted}>{t("assignments.pick-nothing")}</p>
      ) : null}
      {found.hasNextPage ? (
        <Button
          quiet
          isPending={found.isFetchingNextPage}
          onPress={() => {
            void found.fetchNextPage();
          }}
        >
          {t("assignments.pick-more")}
        </Button>
      ) : null}
    </fieldset>
  );
}
