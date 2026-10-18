import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { CustomizeEditable, usedNowText } from "../../shared/editing/index.ts";
import { useCreateLayout, type EditorQuery } from "./layouts.ts";

/** What the form uses now, in words: the layout of another place, or the starting layout. */
function usedNow(editor: Schema<"LayoutEditorBody">): string {
  return usedNowText(editor.inherited_from, {
    starting: t("layouts.not-customized"),
    organization: t("layouts.inherited-organization"),
    hidden: t("layouts.inherited-hidden"),
    project: (project) => t("layouts.inherited-project", { project }),
    type: (type) => t("layouts.inherited-type", { type }),
  });
}

/** A place that has no layout of its own: what it uses now, and *Customize*. */
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
    <CustomizeEditable
      used={usedNow(editor)}
      label={
        project === null || project === undefined
          ? t("layouts.customize-organization")
          : t("layouts.customize-project", { project: project.name })
      }
      create={{
        isPending: create.isPending,
        isError: create.isError,
        error: create.error,
        mutate: () => {
          create.mutate();
        },
      }}
    />
  );
}
