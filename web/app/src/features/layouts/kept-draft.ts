import type { Schema } from "@bugaboo/api-client";

import {
  draftOf,
  nextKey,
  sameLayout,
  withRoles,
  type Draft,
  type Section,
} from "./editor-model.ts";
import { isRecord } from "../../shared/is-record.ts";

/** A section as a kept draft holds it: what a refused save wrote down. */
interface KeptSection {
  readonly tab: string;
  readonly title: string;
  readonly fields: readonly {
    readonly field: string;
    readonly roles: readonly string[];
  }[];
}

function strings(value: unknown): readonly string[] | undefined {
  return Array.isArray(value) && value.every((each) => typeof each === "string")
    ? value
    : undefined;
}

/** The sections a kept draft holds, or nothing when the text is not one. */
function keptSections(text: string): KeptSection[] | undefined {
  let found: unknown;
  try {
    found = JSON.parse(text);
  } catch {
    return undefined;
  }
  if (!Array.isArray(found)) {
    return undefined;
  }
  const sections: KeptSection[] = [];
  for (const each of found as unknown[]) {
    if (!isRecord(each) || typeof each["tab"] !== "string" || typeof each["title"] !== "string") {
      return undefined;
    }
    if (!Array.isArray(each["fields"])) {
      return undefined;
    }
    const placed: { field: string; roles: readonly string[] }[] = [];
    for (const one of each["fields"] as unknown[]) {
      const roles = isRecord(one) ? strings(one["roles"]) : undefined;
      if (!isRecord(one) || typeof one["field"] !== "string" || roles === undefined) {
        return undefined;
      }
      placed.push({ field: one["field"], roles });
    }
    sections.push({ tab: each["tab"], title: each["title"], fields: placed });
  }
  return sections;
}

/**
 * The draft a kept draft makes on the layout as it is now: its sections and the order of its
 * fields, with the roles it ticked (only roles that still exist, and only when the person may
 * choose roles). A field that is not in the editor any more is left out; a field the draft does not
 * hold is listed with the fields that are not on the form.
 */
export function restoredDraft(editor: Schema<"LayoutEditorBody">, text: string): Draft | undefined {
  const sections = keptSections(text);
  if (sections === undefined) {
    return undefined;
  }
  const now = draftOf(editor);
  const rows = new Map(
    [...now.sections.flatMap((section) => section.rows), ...now.available].map((each) => [
      each.field,
      each,
    ]),
  );
  const roleIds = new Set(editor.roles.map((role) => role.id));
  const used = new Set<string>();
  const restored: Section[] = sections.map((section) => ({
    key: nextKey(),
    tab: section.tab,
    title: section.title,
    rows: section.fields.flatMap((placed) => {
      const known = rows.get(placed.field);
      if (known === undefined || used.has(placed.field)) {
        return [];
      }
      used.add(placed.field);
      return [
        editor.choose_roles
          ? withRoles(
              known,
              placed.roles.filter((role) => roleIds.has(role)),
            )
          : known,
      ];
    }),
  }));
  return {
    sections: restored,
    available: [...rows.values()]
      .filter((each) => !used.has(each.field))
      .map((each) => withRoles(each, [])),
  };
}

/** Whether a kept draft holds changes: it is not what the layout shows now (or cannot be read). */
export function differsFromLayout(editor: Schema<"LayoutEditorBody">, text: string): boolean {
  const restored = restoredDraft(editor, text);
  return restored === undefined || !sameLayout(restored, draftOf(editor));
}
