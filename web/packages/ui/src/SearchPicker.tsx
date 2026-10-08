import { createContext, useContext, type ReactNode } from "react";

import { Alert } from "./Alert.tsx";
import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { alertTone, buttonLook, pickerLook, type PickerLook } from "./props.ts";
import { SearchField } from "./SearchField.tsx";
import { part } from "./tokens.ts";

const LookContext = createContext<PickerLook>(pickerLook.list);

export interface PickerMore {
  /** The text of the button, like *Show more people*. */
  readonly label: string;
  /** The next page is on its way. */
  readonly pending: boolean;
  readonly press: () => void;
}

export interface SearchPickerProps {
  readonly look?: PickerLook;
  /** What is chosen here, as the heading of the group. */
  readonly legend: string;
  readonly searchLabel: string;
  /** The name of the button that empties the search box. */
  readonly clearLabel: string;
  /** The name of the list of choices for screen readers. */
  readonly listLabel: string;
  readonly query: string;
  readonly onQuery: (query: string) => void;
  /** The plain message when the choices could not be read; nothing when they could. */
  readonly failed: string | undefined;
  /** What is said when nothing was found for the words typed; nothing while there is something. */
  readonly nothing: string | undefined;
  /** *Show more* under the list, while there is a next page. */
  readonly more: PickerMore | undefined;
  /** The choices: `PickerOption` or `PickerRow` each. */
  readonly children: ReactNode;
}

/**
 * Choosing from a list that is searched by words and read a page at a time: the search box, the
 * choices, what is wrong or missing, and *Show more*. The choices are the children, so one screen
 * picks one thing (radio buttons) and another does something with each (a button per row).
 */
export function SearchPicker({
  look = pickerLook.list,
  legend,
  searchLabel,
  clearLabel,
  listLabel,
  query,
  onQuery,
  failed,
  nothing,
  more,
  children,
}: SearchPickerProps) {
  return (
    <fieldset {...partOf(part.picker)}>
      <legend {...partOf(part.pickerLegend, { variants: [look] })}>{legend}</legend>
      <SearchField label={searchLabel} clearLabel={clearLabel} value={query} onChange={onQuery} />
      <ul {...partOf(part.pickerOptions, { variants: [look] })} aria-label={listLabel}>
        <LookContext value={look}>{children}</LookContext>
      </ul>
      {failed === undefined ? null : <Alert tone={alertTone.danger}>{failed}</Alert>}
      {nothing === undefined ? null : <p {...partOf(part.muted)}>{nothing}</p>}
      {more === undefined ? null : (
        <Button look={buttonLook.quiet} isPending={more.pending} onPress={more.press}>
          {more.label}
        </Button>
      )}
    </fieldset>
  );
}

export interface PickerOptionProps {
  /** The radio buttons of one list share this name. */
  readonly group: string;
  readonly checked: boolean;
  readonly label: string;
  /** More to tell the choices apart, like the short name of a project. */
  readonly hint?: string;
  readonly onPick: () => void;
}

/** One choice of a list of which exactly one is picked. */
export function PickerOption({ group, checked, label, hint, onPick }: PickerOptionProps) {
  const look = useContext(LookContext);
  return (
    <li>
      <label {...partOf(part.pickerOption, { variants: [look] })}>
        <input
          type="radio"
          name={group}
          checked={checked}
          onChange={onPick}
          {...partOf(part.pickerInput, { variants: [look === pickerLook.boxed && look] })}
        />
        <span {...partOf(part.pickerName, { variants: [look] })}>{label}</span>
        {hint === undefined ? null : <PickerHint>{hint}</PickerHint>}
      </label>
    </li>
  );
}

export interface PickerRowProps {
  readonly label: string;
  /** What can be done with the row: a button, or a word that it is done already. */
  readonly children: ReactNode;
}

/** One row of a list of which each can be acted on. */
export function PickerRow({ label, children }: PickerRowProps) {
  return (
    <li {...partOf(part.pickerRow)}>
      <span {...partOf(part.pickerName, { variants: [pickerLook.list] })}>{label}</span>
      {children}
    </li>
  );
}

/** A short word next to a choice that tells it from others. */
export function PickerHint({ children }: { readonly children: ReactNode }) {
  return <span {...partOf(part.pickerHint)}>{children}</span>;
}
