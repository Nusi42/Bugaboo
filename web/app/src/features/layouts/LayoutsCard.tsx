import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { LayoutsOpen } from "../../shared/paths.ts";
import { PlaceEditorCard, placeInherits } from "../projects/index.ts";
import { CustomizeForm } from "./CustomizeForm.tsx";
import { FormChooser } from "./FormChooser.tsx";
import { kindOf, useLayoutEditor, type EditorQuery } from "./layouts.ts";
import { LockedLayout } from "./LockedLayout.tsx";
import { RemoveLayout } from "./RemoveLayout.tsx";

export interface LayoutsCardProps {
  /** The project, by its short name; nothing for the whole organization. */
  readonly project: string | undefined;
  /** The heading of the card. */
  readonly heading: string;
  readonly open: LayoutsOpen;
  /** Opens another form or issue type. */
  readonly onOpen: (open: LayoutsOpen) => void;
}

/** The editor of the layout that is open, once it is read: to customize, or to change. */
function Open({
  editor,
  query,
}: {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly query: EditorQuery;
}) {
  const slot = editor.layout;
  if (slot === null || slot === undefined) {
    return <CustomizeForm editor={editor} query={query} />;
  }
  return (
    <>
      <LockedLayout key={slot.id} editor={editor} layout={slot.id} />
      {placeInherits(query) ? <RemoveLayout slot={slot} /> : null}
    </>
  );
}

/**
 * The editor of the layouts of a place (CFG-01, UX §5.5): which form and issue type, and then the
 * layout, or the way to make one. The same card serves a project and the whole organization.
 */
export function LayoutsCard({ project, heading, open, onOpen }: LayoutsCardProps) {
  const kind = kindOf(open.form);
  const query: EditorQuery = { project, issueType: open.type, kind };
  const editor = useLayoutEditor(query);
  return (
    <PlaceEditorCard
      heading={heading}
      intro={t("layouts.intro")}
      chooser={
        <FormChooser
          kind={kind}
          type={open.type}
          onChange={(nextKind, type) => {
            onOpen({ form: nextKind, type });
          }}
        />
      }
      notes={<p {...partOf(part.muted)}>{t("layouts.fixed")}</p>}
      error={editor.error}
      data={editor.data}
    >
      {(found) => <Open key={[kind, open.type].join("|")} editor={found} query={query} />}
    </PlaceEditorCard>
  );
}
