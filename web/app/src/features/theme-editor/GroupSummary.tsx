import { partOf, part } from "@bugaboo/ui";

/** The heading of a group of the editor: its name, and what the theme changes in it. */
export function GroupSummary({ title, note }: { readonly title: string; readonly note: string }) {
  return (
    <summary {...partOf(part.themeGroupSummary)}>
      <span>{title}</span>
      <span {...partOf(part.themeGroupNote)}>{note}</span>
    </summary>
  );
}
