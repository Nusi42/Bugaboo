import { t } from "@bugaboo/messages";

/** The words around a long text of an issue: its heading and what is shown when it is empty. */
export interface LongTextWords {
  readonly title: string;
  readonly empty: string;
  /** Whether the shortcut *Edit the description* opens this text. */
  readonly shortcut: boolean;
}

/** The ID of the heading of a long text of an issue, for the section to name itself by. */
export function sectionId(issue: string, part: string): string {
  return [issue, part].join("-");
}

/** The words of the description. */
export function descriptionWords(): LongTextWords {
  return {
    title: t("issue-field.description"),
    empty: t("issues.description.empty"),
    shortcut: true,
  };
}
