import { t } from "@bugaboo/messages";
import { Alert, Button, SearchField } from "@bugaboo/ui";
import { useId, useState, type ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";
import styles from "./Access.module.css";
import { peopleOf, placesOf, usePeopleToCheck, useProjectsToChoose } from "./use-check.ts";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

interface OptionProps {
  readonly group: string;
  readonly checked: boolean;
  readonly label: string;
  readonly hint?: string;
  readonly onPick: () => void;
}

/** One choice of a list of choices, as a radio button. */
function Option({ group, checked, label, hint, onPick }: OptionProps) {
  return (
    <li>
      <label className={styles.option}>
        <input type={radio} name={group} checked={checked} onChange={onPick} />
        <span className={styles.optionName}>{label}</span>
        {hint === undefined ? null : <span className={styles.optionHint}>{hint}</span>}
      </label>
    </li>
  );
}

interface ChooserProps {
  readonly legend: string;
  readonly searchLabel: string;
  readonly listLabel: string;
  readonly query: string;
  readonly onQuery: (query: string) => void;
  readonly failed: unknown;
  /** Nothing was found for the words typed. */
  readonly nothing: string | undefined;
  readonly more: {
    readonly label: string;
    readonly pending: boolean;
    readonly press: () => void;
  } | null;
  readonly children: ReactNode;
}

/** The frame of a list to choose from: the search box, the options, what is missing, *Show more*. */
function Chooser({
  legend,
  searchLabel,
  listLabel,
  query,
  onQuery,
  failed,
  nothing,
  more,
  children,
}: ChooserProps) {
  return (
    <fieldset className={styles.fieldset}>
      <legend className={styles.legend}>{legend}</legend>
      <SearchField
        label={searchLabel}
        clearLabel={t("search.clear")}
        value={query}
        onChange={onQuery}
      />
      <ul className={styles.options} aria-label={listLabel}>
        {children}
      </ul>
      {failed === null || failed === undefined ? null : <Alert danger>{messageOf(failed)}</Alert>}
      {nothing === undefined ? null : <p className={styles.muted}>{nothing}</p>}
      {more === null ? null : (
        <Button quiet isPending={more.pending} onPress={more.press}>
          {more.label}
        </Button>
      )}
    </fieldset>
  );
}

export interface PersonPickerProps {
  /** The project the person is checked in (an ID); the whole organization without one. */
  readonly project: string | undefined;
  /** The person picked (an ID), if there is one. */
  readonly value: string | undefined;
  readonly onChange: (person: string) => void;
}

/**
 * The person to check: the people whose names the person who checks may see, found by words. Only
 * names are listed; somebody who may not see names is offered nobody.
 */
export function PersonPicker({ project, value, onChange }: PersonPickerProps) {
  const [query, setQuery] = useState("");
  const group = useId();
  const found = usePeopleToCheck(project, query.trim());
  const people = peopleOf(found.data?.pages);
  return (
    <Chooser
      legend={t("check-access.person-list")}
      searchLabel={t("check-access.person-search")}
      listLabel={t("check-access.person-list")}
      query={query}
      onQuery={setQuery}
      failed={found.error}
      nothing={
        found.isSuccess && people.length === 0 ? t("check-access.person-nothing") : undefined
      }
      more={
        found.hasNextPage
          ? {
              label: t("check-access.person-more"),
              pending: found.isFetchingNextPage,
              press: () => {
                void found.fetchNextPage();
              },
            }
          : null
      }
    >
      {people.map((person) => (
        <Option
          key={person.id}
          group={group}
          checked={value === person.id}
          label={person.name}
          onPick={() => {
            onChange(person.id);
          }}
        />
      ))}
    </Chooser>
  );
}

export interface PlacePickerProps {
  /** The project picked (an ID); `null` for the whole organization. */
  readonly value: string | null;
  readonly onChange: (project: string | null) => void;
}

/** Where to look: the whole organization, or one of the projects the person sees, found by words. */
export function PlacePicker({ value, onChange }: PlacePickerProps) {
  const [query, setQuery] = useState("");
  const group = useId();
  const found = useProjectsToChoose(query.trim());
  const places = placesOf(found.data?.pages);
  return (
    <Chooser
      legend={t("check-access.place-legend")}
      searchLabel={t("check-access.place-search")}
      listLabel={t("check-access.place-list")}
      query={query}
      onQuery={setQuery}
      failed={found.error}
      nothing={found.isSuccess && places.length === 0 ? t("check-access.place-nothing") : undefined}
      more={
        found.hasNextPage
          ? {
              label: t("check-access.place-more"),
              pending: found.isFetchingNextPage,
              press: () => {
                void found.fetchNextPage();
              },
            }
          : null
      }
    >
      <Option
        group={group}
        checked={value === null}
        label={t("check-access.place-everywhere")}
        onPick={() => {
          onChange(null);
        }}
      />
      {places.map((place) => (
        <Option
          key={place.id}
          group={group}
          checked={value === place.id}
          label={place.name}
          hint={place.shortName}
          onPick={() => {
            onChange(place.id);
          }}
        />
      ))}
    </Chooser>
  );
}
