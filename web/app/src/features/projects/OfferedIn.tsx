import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  PickerOption,
  SearchPicker,
  SelectField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { pageSize } from "../../shared/paged.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { linesOf, useProjectList } from "./use-projects.ts";

/** One line that says where something is offered, as the server sends it (the fields can be missing). */
export type OfferLine = Schema<"OfferRowItem">;

/** A line to change: a project (an ID) or the whole organization (`null`), and what it says. */
export interface OfferChangeLine {
  readonly project: string | null;
  /** Offered (`true`), not offered (`false`), or the line is taken away (`null`). */
  readonly offered: boolean | null;
}

export interface OfferedInProps {
  /** The lines there are. */
  readonly rows: readonly OfferLine[];
  readonly isPending: boolean;
  /** What went wrong with the last change, in words. */
  readonly error: string | null;
  readonly onChange: (change: OfferChangeLine) => void;
}

/** The project a place names (an ID), the whole organization (`null`), or nothing known. */
function projectOf(place: OfferLine["place"]): string | null | undefined {
  if (place === undefined || place === null || place.kind === "hidden") {
    return undefined;
  }
  return place.kind === "project" ? place.id : null;
}

/** What a place is called in a sentence. */
function placeName(place: OfferLine["place"]): string {
  if (place === undefined || place === null || place.kind === "hidden") {
    return t("projects.offer-hidden-project");
  }
  return place.kind === "project" ? place.name : t("projects.offer-whole-organization");
}

/** What a line says about the place: offered or not. */
function stateName(offered: boolean | null | undefined): string {
  return offered === true ? t("projects.offer-state-offered") : t("projects.offer-state-not");
}

/** A place the person picked to add a line for. */
interface Picked {
  readonly project: string | null;
  readonly name: string;
}

/** The projects the person finds by words; picking one is the place of the new line. */
function PlacePicker({ onPick }: { readonly onPick: (picked: Picked) => void }) {
  const { words, group, frame } = usePickerFrame();
  const found = useProjectList({ search: words }, pageSize.short);
  const projects = linesOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("projects.offered-in-pick-legend")}
      searchLabel={t("projects.offered-in-pick-search")}
      listLabel={t("projects.offered-in-pick-list")}
      {...frame}
      {...pickerStatus(found, projects.length, {
        nothing: t("projects.offered-in-pick-nothing"),
        more: t("projects.offered-in-pick-more"),
      })}
    >
      {projects.map((project) => (
        <PickerOption
          key={project.id}
          group={group}
          checked={false}
          label={project.name}
          hint={project.shortName}
          onPick={() => {
            onPick({ project: project.id, name: project.name });
          }}
        />
      ))}
    </SearchPicker>
  );
}

const OFFERED = "offered";
const NOT_OFFERED = "not-offered";

/** The picked place, whether it is offered there, and the button that saves the line. */
function NewLine({
  picked,
  isPending,
  onSave,
}: {
  readonly picked: Picked;
  readonly isPending: boolean;
  readonly onSave: (offered: boolean) => void;
}) {
  const [state, setState] = useState<typeof OFFERED | typeof NOT_OFFERED>(OFFERED);
  return (
    <>
      <p {...partOf(part.text)}>{t("projects.offered-in-picked", { place: picked.name })}</p>
      <SelectField
        label={t("projects.offered-in-state")}
        choices={[
          { value: OFFERED, label: t("projects.offer-state-offered") },
          { value: NOT_OFFERED, label: t("projects.offer-state-not") },
        ]}
        value={state}
        onChange={setState}
      />
      <div {...partOf(part.row)}>
        <Button
          isPending={isPending}
          onPress={() => {
            onSave(state === OFFERED);
          }}
        >
          {t("projects.offered-in-save")}
        </Button>
      </div>
    </>
  );
}

/** The form that adds a line: pick a place, then say whether it is offered there. */
function AddLine({
  isPending,
  onChange,
}: {
  readonly isPending: boolean;
  readonly onChange: (change: OfferChangeLine) => void;
}) {
  const [picked, setPicked] = useState<Picked>();
  return (
    <div {...partOf(part.stack)}>
      <h4 {...partOf(part.fieldLabel)}>{t("projects.offered-in-add")}</h4>
      <PlacePicker onPick={setPicked} />
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setPicked({ project: null, name: t("projects.offer-whole-organization") });
          }}
        >
          {t("projects.offer-whole-organization")}
        </Button>
      </div>
      {picked === undefined ? (
        <p {...partOf(part.muted)}>{t("projects.offered-in-pick-first")}</p>
      ) : (
        <NewLine
          key={picked.project ?? "organization"}
          picked={picked}
          isPending={isPending}
          onSave={(offered) => {
            onChange({ project: picked.project, offered });
            setPicked(undefined);
          }}
        />
      )}
    </div>
  );
}

/** The lines that say where it is offered, each with a button that takes it away. */
function Lines({
  rows,
  isPending,
  onChange,
}: {
  readonly rows: readonly OfferLine[];
  readonly isPending: boolean;
  readonly onChange: (change: OfferChangeLine) => void;
}) {
  return (
    <ul {...partOf(part.list)} aria-label={t("projects.offered-in-list")}>
      {rows.map((row) => {
        const name = placeName(row.place);
        const project = projectOf(row.place);
        return (
          <li key={project ?? "organization"} {...partOf(part.item)}>
            <span>{t("projects.offer-line", { name, state: stateName(row.offered) })}</span>
            {row.may_change === true && project !== undefined ? (
              <Button
                look={buttonLook.quiet}
                aria-label={t("projects.offered-in-take-away-for", { place: name })}
                isPending={isPending}
                onPress={() => {
                  onChange({ project, offered: null });
                }}
              >
                {t("projects.offered-in-take-away")}
              </Button>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}

/**
 * *Offered in* (ISS-02, TAG-02) for an issue type or a group of labels in *Organization settings*:
 * the lines that say where it is offered, each with a button that takes it away, and the form that
 * adds one. Without a line it is offered in every project.
 */
export function OfferedIn({ rows, isPending, error, onChange }: OfferedInProps) {
  return (
    <section {...partOf(part.stack)} aria-label={t("projects.offered-in-title")}>
      <h3 {...partOf(part.accessTopic)}>{t("projects.offered-in-title")}</h3>
      {rows.length === 0 ? (
        <p {...partOf(part.text)}>{t("projects.offered-in-everywhere")}</p>
      ) : (
        <>
          <p {...partOf(part.muted)}>{t("projects.offered-in-rows")}</p>
          <Lines rows={rows} isPending={isPending} onChange={onChange} />
        </>
      )}
      {error === null ? null : <Alert tone={alertTone.danger}>{error}</Alert>}
      <AddLine isPending={isPending} onChange={onChange} />
    </section>
  );
}
