import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { fieldNameOf } from "../../shared/field-names.ts";
import { useNeverSent } from "./assistant.ts";

/**
 * *Never sent to AI:* with the fields and projects an administrator marked (AI-03, UX §4.12). They
 * stay out of every answer, also for the people who can read them.
 */
export function NeverSent() {
  const marks = useNeverSent().data;
  if (marks === undefined) {
    return null;
  }
  const names = [
    ...marks.fields.filter((each) => each.marked).map((each) => fieldNameOf({ code: each.field })),
    ...marks.projects.filter((each) => each.marked).map((each) => each.project.name),
  ];
  return (
    <p {...partOf(part.muted)}>
      {names.length === 0
        ? t("assistant.never-sent-none")
        : t("assistant.never-sent", { names: names.join(", ") })}
    </p>
  );
}
