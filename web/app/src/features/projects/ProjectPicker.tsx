import { t } from "@bugaboo/messages";
import { Alert, Button, SearchField } from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import styles from "./Projects.module.css";
import { linesOf, SEARCH_SIZE, useProjectList, type ProjectLine } from "./use-projects.ts";

/** Words of the HTML standard, not text for people. */
const radio = "radio";

export interface ProjectPickerProps {
  /** The project that moves: it, and what is inside it, are not offered. */
  readonly moving: string;
  /** Where it goes: another project (an ID), the top level (`null`), or nothing picked yet. */
  readonly value: string | null | undefined;
  readonly onChange: (value: string | null) => void;
}

interface OptionProps {
  readonly group: string;
  readonly checked: boolean;
  readonly label: string;
  readonly hint?: string;
  readonly onPick: () => void;
}

/** One place the project can go, as a radio button. */
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

interface OptionListProps extends Pick<ProjectPickerProps, "value" | "onChange"> {
  readonly projects: readonly ProjectLine[];
}

/** The top level, then the projects found. */
function OptionList({ projects, value, onChange }: OptionListProps) {
  const group = useId();
  return (
    <ul className={styles.options} aria-label={t("projects.picker-list")}>
      <Option
        group={group}
        checked={value === null}
        label={t("projects.picker-top")}
        onPick={() => {
          onChange(null);
        }}
      />
      {projects.map((project) => (
        <Option
          key={project.id}
          group={group}
          checked={value === project.id}
          label={project.name}
          hint={project.shortName}
          onPick={() => {
            onChange(project.id);
          }}
        />
      ))}
    </ul>
  );
}

/**
 * Where a project goes (PRJ-03): the top level, or one of the projects the person finds by
 * searching. The project itself and what is inside it are left out; the server decides again.
 */
export function ProjectPicker({ moving, value, onChange }: ProjectPickerProps) {
  const [query, setQuery] = useState("");
  const found = useProjectList({ search: query.trim(), outside: moving }, true, SEARCH_SIZE);
  const projects = linesOf(found.data?.pages);
  return (
    <fieldset className={styles.picker}>
      <legend className={styles.legend}>{t("projects.picker-legend")}</legend>
      <SearchField
        label={t("projects.picker-search")}
        clearLabel={t("search.clear")}
        value={query}
        onChange={setQuery}
      />
      <OptionList projects={projects} value={value} onChange={onChange} />
      {found.isError ? <Alert danger>{messageOf(found.error)}</Alert> : null}
      {found.isSuccess && projects.length === 0 ? (
        <p className={styles.muted}>{t("projects.picker-nothing")}</p>
      ) : null}
      {found.hasNextPage ? (
        <Button
          quiet
          isPending={found.isFetchingNextPage}
          onPress={() => {
            void found.fetchNextPage();
          }}
        >
          {t("projects.picker-more")}
        </Button>
      ) : null}
    </fieldset>
  );
}
