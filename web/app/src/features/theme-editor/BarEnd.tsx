import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { UnsavedNote } from "./EditorNotes.tsx";
import type { EditorBarProps } from "./EditorBar.tsx";
import type { useSaving } from "./use-save-theme.ts";

/**
 * The end of the bar: *Not saved yet* or *Saved* (UX §6.4), what a phone adds, *Throw my changes
 * away* and *Save theme*. A theme for everyone whose save was refused for colors that are hard to
 * read cannot be saved until they read well: *Save theme* is off, and the readability on top says why.
 */
export function BarEnd({
  detail,
  editing,
  saving,
  extra,
}: Pick<EditorBarProps, "detail" | "editing" | "extra"> & {
  readonly saving: ReturnType<typeof useSaving>;
}) {
  return (
    <div {...partOf(part.themeEditorBarEnd)}>
      <UnsavedNote dirty={editing.dirty} saved={saving.saved} />
      {extra}
      <Button
        look={buttonLook.quiet}
        isDisabled={!editing.dirty}
        onPress={() => {
          editing.setValues(detail.values);
          editing.setName(detail.theme.name);
        }}
      >
        {t("editing.changes-drop")}
      </Button>
      <Button
        look={buttonLook.primary}
        isPending={saving.pending}
        isDisabled={!editing.dirty || editing.refused}
        onPress={saving.save}
      >
        {t("theme-editor.save")}
      </Button>
    </div>
  );
}
