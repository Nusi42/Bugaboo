import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  BottomSheet,
  Button,
  buttonLook,
  doNothing,
  IssueId,
  LabelChip,
  pageAttributes,
  partOf,
  part,
  StatusPill,
  TextField,
  usePreviewSheet,
} from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

/** The sample issue of the preview: in progress, with two tags of different colors. */
const SAMPLE = { status: "in-progress", firstTag: "cat2", secondTag: "cat7" } as const;

export interface EditorPreviewProps {
  readonly theme: Schema<"ThemeBody">;
  /** Whether the sheet of the person's draft is there yet; until then the preview shows the saved theme. */
  readonly draftShown: boolean;
}

/**
 * The attributes of the preview: the sample of the built-in theme the own theme starts from, with
 * the person's draft over it, or before there is one the saved theme (its sheet reaches a sample
 * that names it). Never both, so a color the draft takes back to the built-in one does not show
 * the saved one.
 */
function lookOf(theme: Schema<"ThemeBody">, draftShown: boolean) {
  if (draftShown) {
    return { [pageAttributes.previewLook]: theme.extends, [pageAttributes.ownDraft]: theme.id };
  }
  return theme.saved
    ? { [pageAttributes.previewLook]: theme.extends, [pageAttributes.ownTheme]: theme.id }
    : { [pageAttributes.previewLook]: theme.extends };
}

/**
 * The live preview of the theme editor (UX §5.6): real parts of the app (a row of an issue with its
 * status and tags, a field, buttons, notes) drawn under the theme being edited. It is a sample of
 * the built-in theme (`data-preview-look`) that names the person's draft (`data-own-draft`) or, until
 * there is one, the saved theme (`data-own-theme`), whose sheets the server makes, so no style is
 * made in the page and nobody else sees the draft before *Save*.
 * It is a picture of the theme: its parts take no keys and say nothing to a screen reader, and it
 * shows a pair that is hard to read as it is; the line of the readability on top says it in words.
 */
export function EditorPreview({ theme, draftShown }: EditorPreviewProps) {
  usePreviewSheet(draftShown ? undefined : (theme.sheet ?? undefined));
  return (
    <div {...partOf(part.themeEditorPreview)} {...lookOf(theme, draftShown)} inert aria-hidden>
      <div {...partOf(part.themeEditorSampleRow)}>
        <IssueId id={t("theme-editor.sample.id")} />
        <span>{t("theme-editor.sample.title")}</span>
        <StatusPill group={SAMPLE.status} label={t("theme-editor.sample.status")} />
        <LabelChip text={t("theme-editor.sample.tag-sign-in")} color={SAMPLE.firstTag} />
        <LabelChip text={t("theme-editor.sample.tag-crash")} color={SAMPLE.secondTag} />
      </div>
      <p {...partOf(part.muted)}>{t("theme-editor.sample.quiet")}</p>
      <p {...partOf(part.themeEditorSampleHint)}>{t("theme-editor.sample.hint")}</p>
      <TextField
        label={t("theme-editor.sample.field")}
        value={t("theme-editor.sample.title")}
        onChange={doNothing}
      />
      <div {...partOf(part.row)}>
        <Button look={buttonLook.primary} onPress={doNothing}>
          {t("theme-editor.sample.button")}
        </Button>
        <Button onPress={doNothing}>{t("theme-editor.sample.other-button")}</Button>
      </div>
      <Alert tone={alertTone.warning}>{t("theme-editor.sample.warning")}</Alert>
      <Alert tone={alertTone.success}>{t("theme-editor.sample.success")}</Alert>
    </div>
  );
}

/**
 * On a phone the preview opens from *Preview* in the bar at the foot (UX §5.6); in the sheet only
 * the preview scrolls, so its foot with *Close* stays in sight.
 */
export function PhonePreview({ children }: { readonly children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("theme-editor.preview")}
      </Button>
      <BottomSheet
        isOpen={open}
        onOpenChange={setOpen}
        label={t("theme-editor.preview")}
        title={t("theme-editor.preview")}
        closeLabel={t("common.close")}
        tall
      >
        <div {...partOf(part.themeEditorPreviewScroll)}>{children}</div>
      </BottomSheet>
    </>
  );
}
