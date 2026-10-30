import { createContext, useContext, type ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";

import { Alert } from "./Alert.tsx";
import { partOf } from "./parts.ts";
import { alertTone, pickerLook, type PickerLook } from "./props.ts";
import { SearchField } from "./SearchField.tsx";
import { ShowMore } from "./ShowMore.tsx";
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
      {/* The legend is the one visible label; the box says what to type in it, and screen readers read both. */}
      <SearchField
        label={searchLabel}
        hideLabel
        placeholder={searchLabel}
        clearLabel={clearLabel}
        value={query}
        onChange={onQuery}
      />
      <ul
        {...(look === pickerLook.boxed
          ? partOf(part.list, { variants: ["tight"] })
          : partOf(part.pickerOptions, { variants: [pickerLook.list] }))}
        aria-label={listLabel}
      >
        <LookContext value={look}>{children}</LookContext>
      </ul>
      {failed === undefined ? null : <Alert tone={alertTone.danger}>{failed}</Alert>}
      {nothing === undefined ? null : <p {...partOf(part.muted)}>{nothing}</p>}
      {more === undefined ? null : (
        <ShowMore label={more.label} pending={more.pending} onPress={more.press} />
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

export interface PickerChoiceProps {
  readonly label: ReactNode;
  /** The name for screen readers, when the label alone does not say what the press does; the words of the label when left out. */
  readonly accessibleName?: string;
  /** More to read in the same row, on the same line (a status, an ID). */
  readonly children?: ReactNode;
  readonly onPick: () => void;
}

/**
 * One row of a list of which one is taken with a press: the whole row is the button, and the name
 * is underlined when it is pointed at (no column of *Choose* links).
 */
export function PickerChoice({ label, accessibleName, children, onPick }: PickerChoiceProps) {
  return (
    <li>
      <AriaButton
        {...partOf(part.pickerChoice)}
        {...(accessibleName === undefined ? {} : { "aria-label": accessibleName })}
        onPress={onPick}
      >
        <span {...partOf(part.pickerName, { variants: [pickerLook.list] })}>{label}</span>
        {children}
      </AriaButton>
    </li>
  );
}

/** The ID of an issue in a row of a picker, in the font of IDs. */
export function PickerId({ children }: { readonly children: ReactNode }) {
  return <span {...partOf(part.pickerId)}>{children}</span>;
}
