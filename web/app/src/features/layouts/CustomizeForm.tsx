import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useCreateLayout, type EditorQuery } from "./layouts.ts";

/** What the form uses now, in words: the layout of another place, or the starting layout. */
function usedNow(editor: Schema<"LayoutEditorBody">): string {
  const from = editor.inherited_from;
  if (from === null || from === undefined) {
    return t("layouts.not-customized");
  }
  const where = from.whole_organization
    ? t("layouts.inherited-organization")
    : from.project === null || from.project === undefined
      ? t("layouts.inherited-hidden")
      : t("layouts.inherited-project", { project: from.project.name });
  return from.issue_type === null || from.issue_type === undefined
    ? where
    : `${where} ${t("layouts.inherited-type", { type: from.issue_type.name })}`;
}

/**
 * A place that has no layout of its own: what it uses now, and *Customize* that makes one as a copy
 * of it, so the person can change it without touching the place it comes from.
 */
export function CustomizeForm({
  editor,
  query,
}: {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly query: EditorQuery;
}) {
  const create = useCreateLayout(query);
  const project = editor.project;
  return (
    <div {...partOf(part.stack)}>
      <p>{usedNow(editor)}</p>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.primary}
          isPending={create.isPending}
          onPress={() => {
            create.mutate();
          }}
        >
          {project === null || project === undefined
            ? t("layouts.customize-organization")
            : t("layouts.customize-project", { project: project.name })}
        </Button>
      </div>
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
    </div>
  );
}
