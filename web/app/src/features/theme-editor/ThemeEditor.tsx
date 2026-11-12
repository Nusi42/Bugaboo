import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part, TextField, useLayout } from "@bugaboo/ui";

import { DraftNote, type EditLock } from "../../shared/editing/index.ts";
import { ColorGroups } from "./ColorGroups.tsx";
import { EditorPreview, PhonePreview } from "./EditorPreview.tsx";
import { HardPairs, ReadabilityLine } from "./ReadabilityLine.tsx";
import { SaveTheme } from "./SaveTheme.tsx";
import { useThemeEditing, type Editing } from "./use-theme-editing.ts";

/** What stands on top: the line of the readability, the pairs that are hard to read, a kept draft. */
function EditorTop({
  editing,
  kind,
}: {
  readonly editing: Editing;
  readonly kind: Schema<"ThemeKind">;
}) {
  return (
    <>
      <ReadabilityLine summary={editing.read?.summary} kind={kind} />
      {editing.hard.length === 0 ? null : <HardPairs pairs={editing.hard} onShow={editing.show} />}
      {editing.kept.kept ? (
        <DraftNote
          text={t("theme-editor.draft-kept")}
          drop={{ label: t("editing.draft-drop"), onPress: editing.kept.drop }}
        />
      ) : null}
    </>
  );
}

export interface ThemeEditorProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly lock: EditLock;
}

/**
 * The theme editor (UX §5.6, THM-03): the line of the readability on top, the groups of colors and
 * *Shadow, corners and fonts*, and the live preview through the sheet of the person's draft, beside
 * them on a computer and behind *Preview* on a phone. The values on screen are kept as the draft
 * while the person types, so nothing typed is dropped, and nobody else sees them before *Save*
 * (ADR-0019, ADR-0026 update M8). While somebody else holds the lock only the preview is shown.
 */
export function ThemeEditor({ detail, lock }: ThemeEditorProps) {
  const { theme } = detail;
  const editing = useThemeEditing(detail, lock);
  const phone = useLayout() === "phone";
  const preview = (
    <EditorPreview theme={theme.id} startsFrom={theme.extends} draftShown={editing.draftShown} />
  );
  return (
    <div {...partOf(part.themeEditor)}>
      <EditorTop editing={editing} kind={theme.kind} />
      <div {...partOf(part.themeEditorBody)}>
        {editing.mine ? (
          <div {...partOf(part.themeEditorGroups)}>
            <TextField
              label={t("theme-editor.name")}
              requiredMessage={t("error.field.required")}
              value={editing.name}
              onChange={editing.setName}
            />
            <ColorGroups editing={editing} startsFrom={theme.extends} />
          </div>
        ) : null}
        {phone ? null : <div {...partOf(part.themeEditorAside)}>{preview}</div>}
      </div>
      {editing.mine ? <SaveTheme detail={detail} editing={editing} lock={lock} /> : null}
      {phone ? <PhonePreview>{preview}</PhonePreview> : null}
    </div>
  );
}
