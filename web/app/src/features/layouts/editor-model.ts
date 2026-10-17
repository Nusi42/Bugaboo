import type { Schema } from "@bugaboo/api-client";

import { newId } from "../../shared/new-ids.ts";

/** One field in the editor: its key, name (own fields), the roles it is shown to and the marks. */
export interface Row {
  /** The key of the field: the word of a built-in field or the ID of an own field. */
  readonly field: string;
  /** The name of an own field; a built-in field is named by its word. */
  readonly name: string | null;
  /** The IDs of the roles the layout shows the field to. */
  readonly roles: readonly string[];
  /** The roles for which field permissions win. */
  readonly limitedFor: readonly string[];
  /** An own field that must be filled in. */
  readonly required: boolean;
}

/** One section in the editor; `key` only tells sections apart while they are moved. */
export interface Section {
  readonly key: string;
  readonly tab: string;
  readonly title: string;
  readonly rows: readonly Row[];
}

/** What the editor changes: the sections, and the fields that are on no section. */
export interface Draft {
  readonly sections: readonly Section[];
  readonly available: readonly Row[];
}

/** Where a field goes: a section, and its place in it. */
export interface Target {
  readonly section: string;
  readonly index: number;
}

/** A key for a section that is not used by another. */
export function nextKey(): string {
  return newId("section");
}

function rowOf(field: Schema<"EditorFieldBody">): Row {
  return {
    field: field.field,
    name: field.name ?? null,
    roles: field.roles,
    limitedFor: field.limited_for,
    required: field.required,
  };
}

/** The draft an editor view starts from. */
export function draftOf(editor: Schema<"LayoutEditorBody">): Draft {
  return {
    sections: editor.sections.map((section) => ({
      key: nextKey(),
      tab: section.tab,
      title: section.title,
      rows: section.fields.map(rowOf),
    })),
    available: editor.available.map(rowOf),
  };
}

/** The sections as the API takes them. */
export function bodyOf(draft: Draft): Schema<"SaveLayoutBody">["sections"] {
  return draft.sections.map((section) => ({
    tab: section.tab,
    title: section.title,
    fields: section.rows.map((row) => ({
      field: row.field,
      roles: [...row.roles],
    })),
  }));
}

/** Whether two drafts would save the same layout. */
export function sameLayout(a: Draft, b: Draft): boolean {
  return JSON.stringify(bodyOf(a)) === JSON.stringify(bodyOf(b));
}

/** The section the field is in now; nothing for a field that is on no section. */
export function sectionOf(draft: Draft, field: string): Section | undefined {
  return draft.sections.find((section) => section.rows.some((row) => row.field === field));
}

/** The row of the field and the section it is in (none for an available field). */
function find(draft: Draft, field: string): Row | undefined {
  return (
    draft.sections.flatMap((section) => section.rows).find((row) => row.field === field) ??
    draft.available.find((row) => row.field === field)
  );
}

/** The section with other rows. */
function withRows(section: Section, rows: readonly Row[]): Section {
  return { ...section, rows };
}

/** The row with other roles. */
export function withRoles(row: Row, roles: readonly string[]): Row {
  return { ...row, roles };
}

/** The draft without the field, wherever it was. */
function without(draft: Draft, field: string): Draft {
  return {
    sections: draft.sections.map((section) =>
      withRows(
        section,
        section.rows.filter((row) => row.field !== field),
      ),
    ),
    available: draft.available.filter((row) => row.field !== field),
  };
}

/**
 * Puts a field into a section at a place. A field that is on no section starts with every role in
 * `everyone` ticked; one that moves keeps its roles. Where the field was before, the place counts
 * after it has been taken out, so moving down by one is `index + 1` of the old place.
 */
export function placeField(
  draft: Draft,
  field: string,
  target: Target,
  everyone: readonly string[],
): Draft {
  const row = find(draft, field);
  const wasHere = sectionOf(draft, field) !== undefined;
  if (row === undefined) {
    return draft;
  }
  const moving: Row = wasHere ? row : withRoles(row, everyone);
  const rest = without(draft, field);
  return {
    available: rest.available,
    sections: rest.sections.map((section) => {
      if (section.key !== target.section) {
        return section;
      }
      const at = Math.max(0, Math.min(target.index, section.rows.length));
      return withRows(section, [...section.rows.slice(0, at), moving, ...section.rows.slice(at)]);
    }),
  };
}

/** Moves a field one place up or down inside its section. */
export function nudgeField(draft: Draft, field: string, by: -1 | 1): Draft {
  const section = sectionOf(draft, field);
  if (section === undefined) {
    return draft;
  }
  const at = section.rows.findIndex((row) => row.field === field);
  const to = at + by;
  if (to < 0 || to >= section.rows.length) {
    return draft;
  }
  return placeField(draft, field, { section: section.key, index: to }, []);
}

/** Takes a field off the form: it is listed with the fields that are not on it. */
export function unplaceField(draft: Draft, field: string): Draft {
  const row = find(draft, field);
  if (row === undefined || sectionOf(draft, field) === undefined) {
    return draft;
  }
  const rest = without(draft, field);
  return { ...rest, available: [...rest.available, withRoles(row, [])] };
}

/** The roles after one is ticked or unticked. */
function ticked(roles: readonly string[], role: string, on: boolean): readonly string[] {
  if (!on) {
    return roles.filter((each) => each !== role);
  }
  return roles.includes(role) ? roles : [...roles, role];
}

/** Ticks or unticks a role for a field. */
export function setRole(draft: Draft, field: string, role: string, on: boolean): Draft {
  return {
    available: draft.available,
    sections: draft.sections.map((section) =>
      withRows(
        section,
        section.rows.map((row) =>
          row.field === field ? withRoles(row, ticked(row.roles, role, on)) : row,
        ),
      ),
    ),
  };
}

/** Adds an empty section at the end. */
export function addSection(draft: Draft): Draft {
  return {
    ...draft,
    sections: [...draft.sections, { key: nextKey(), tab: "", title: "", rows: [] }],
  };
}

/** The section with another title or tab. */
function changed(
  section: Section,
  change: { readonly title?: string; readonly tab?: string },
): Section {
  return { ...section, ...change };
}

/** Changes the title or the tab of a section. */
export function editSection(
  draft: Draft,
  key: string,
  change: { readonly title?: string; readonly tab?: string },
): Draft {
  return {
    ...draft,
    sections: draft.sections.map((section) =>
      section.key === key ? changed(section, change) : section,
    ),
  };
}

/** Takes a section away; its fields are listed with the fields that are not on the form. */
export function removeSection(draft: Draft, key: string): Draft {
  const gone = draft.sections.find((section) => section.key === key);
  if (gone === undefined) {
    return draft;
  }
  return {
    sections: draft.sections.filter((section) => section.key !== key),
    available: [...draft.available, ...gone.rows.map((row) => withRoles(row, []))],
  };
}

/** Moves a section one place up or down. */
export function nudgeSection(draft: Draft, key: string, by: -1 | 1): Draft {
  const at = draft.sections.findIndex((section) => section.key === key);
  const to = at + by;
  const section = draft.sections[at];
  if (section === undefined || to < 0 || to >= draft.sections.length) {
    return draft;
  }
  const rest = draft.sections.filter((each) => each.key !== key);
  return {
    ...draft,
    sections: [...rest.slice(0, to), section, ...rest.slice(to)],
  };
}
