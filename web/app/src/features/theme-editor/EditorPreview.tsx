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
} from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

/** The sample issue of the preview: in progress, with two tags of different colors. */
const SAMPLE = { status: "in-progress", firstTag: "cat2", secondTag: "cat7" } as const;

export interface EditorPreviewProps {
  readonly theme: string;
  /** The built-in theme the own theme starts from: the sample the draft's sheet is laid over. */
  readonly startsFrom: string;
  /** Whether the sheet of the person's draft is there yet; until then the preview shows the built-in theme. */
  readonly draftShown: boolean;
}

/**
 * The live preview of the theme editor (UX §5.6): real parts of the app (a row of an issue with its
 * status and tags, a field, buttons, notes) drawn under the theme being edited. It is a sample of
 * the built-in theme (`data-preview-look`) that names the person's draft (`data-own-draft`), whose
 * sheet the server makes, so no style is made in the page and nobody else sees it before *Save*.
 * It is a picture of the theme: its parts take no keys and say nothing to a screen reader, and it
 * shows a pair that is hard to read as it is; the line of the readability on top says it in words.
 */
export function EditorPreview({ theme, startsFrom, draftShown }: EditorPreviewProps) {
  const attributes = {
    [pageAttributes.previewLook]: startsFrom,
    ...(draftShown ? { [pageAttributes.ownDraft]: theme } : {}),
  };
  return (
    <div {...partOf(part.themeEditorPreview)} {...attributes} inert aria-hidden>
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

/** On a phone the preview opens from a *Preview* button at the bottom (UX §5.6). */
export function PhonePreview({ children }: { readonly children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        look={buttonLook.quiet}
        wide
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
        {children}
      </BottomSheet>
    </>
  );
}
