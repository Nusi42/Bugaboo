import type { Schema } from "@bugaboo/api-client";
import { Card, partOf, part, useLayout } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { ColorGroups } from "./ColorGroups.tsx";
import { EditorBar } from "./EditorBar.tsx";
import { EditorPreview, PhonePreview } from "./EditorPreview.tsx";
import { ReadabilityLine } from "./ReadabilityLine.tsx";
import { KeptDraftNote, UnsavedNote } from "./EditorNotes.tsx";
import { useThemeEditing } from "./use-theme-editing.ts";

export interface ThemeEditorProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly lock: EditLock;
  /** The notes about the edit lock, under the title. */
  readonly notices: ReactNode;
}

/**
 * The theme editor (UX §5.6, THM-03): the title with whether the changes are saved, the readability
 * on top, the groups of colors and *Shadow, corners and fonts*, the live preview beside them on a
 * computer (behind *Preview* on a phone) and one bar at the foot. The values on screen are kept as
 * the person's draft while they type, so nothing typed is dropped, and nobody else sees them before
 * *Save* (ADR-0019, ADR-0026 update M8). While somebody else holds the lock only the preview shows.
 */
export function ThemeEditor({ detail, lock, notices }: ThemeEditorProps) {
  const { theme } = detail;
  const editing = useThemeEditing(detail, lock);
  const phone = useLayout() === "phone";
  const { dirty } = editing;
  const preview = <EditorPreview theme={theme} draftShown={editing.draftShown} />;
  return (
    <Card
      heading={theme.name}
      level={1}
      {...(editing.mine && dirty ? { actions: <UnsavedNote /> } : {})}
    >
      {notices}
      <div {...partOf(part.themeEditor)}>
        <ReadabilityLine
          read={editing.read}
          kind={theme.kind}
          refused={editing.refused}
          onChange={editing.show}
        />
        <KeptDraftNote kept={editing.kept} />
        <div {...partOf(part.themeEditorBody)}>
          {editing.mine ? (
            <div {...partOf(part.themeEditorGroups)}>
              <ColorGroups editing={editing} startsFrom={theme.extends} />
            </div>
          ) : null}
          {phone && editing.mine ? null : <div {...partOf(part.themeEditorAside)}>{preview}</div>}
        </div>
        {editing.mine ? (
          <EditorBar
            detail={detail}
            editing={editing}
            lock={lock}
            extra={phone ? <PhonePreview>{preview}</PhonePreview> : null}
          />
        ) : null}
      </div>
    </Card>
  );
}
