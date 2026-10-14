import type { FormSection } from "./layouts.ts";

/** The sections under one tab; the tab `""` holds the sections that are under none. */
export interface TabGroup {
  readonly tab: string;
  readonly sections: readonly FormSection[];
}

/**
 * The sections grouped by their tab, in the order the tabs first appear. A layout without tabs is
 * one group, so the form is drawn without any.
 */
export function tabsOf(sections: readonly FormSection[]): readonly TabGroup[] {
  const order: string[] = [];
  const under = new Map<string, FormSection[]>();
  for (const section of sections) {
    const list = under.get(section.tab);
    if (list === undefined) {
      order.push(section.tab);
      under.set(section.tab, [section]);
    } else {
      list.push(section);
    }
  }
  return order.map((tab) => ({ tab, sections: under.get(tab) ?? [] }));
}

/**
 * The sections with only the fields `keep` says yes to; a section left without a field is
 * dropped. A place that draws only some of the fields of the layout (the details of an issue)
 * asks for these.
 */
export function onlyFields(
  sections: readonly FormSection[],
  keep: (field: string) => boolean,
): readonly FormSection[] {
  return sections
    .map((section): FormSection => ({
      tab: section.tab,
      title: section.title,
      fields: section.fields.filter((field) => keep(field)),
    }))
    .filter((section) => section.fields.length > 0);
}
