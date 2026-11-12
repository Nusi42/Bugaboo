import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, FilePicker, ownThemeUploadMost } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { refusedValuesOf } from "./refusals.ts";
import { asFileText, useUploadTheme } from "./themes.ts";
import { themeEditorAddress } from "./addresses.ts";

/** Bytes in a kilobyte, as the limit of a file is said: 64 KB. */
const BYTES_IN_KB = 1024;

/**
 * *Upload*: makes an own theme of `kind` from a file a *Download* gave. The text of the file goes to
 * the server as it is, which judges all of it and names every part and value it refuses; a file
 * larger than the server takes is refused before it is read.
 */
/** What an upload is doing: the file it sends, and why a file was not sent. */
function useUpload(kind: Schema<"ThemeKind">) {
  const navigate = useNavigate();
  const upload = useUploadTheme();
  const [unread, setUnread] = useState(false);
  const [tooLarge, setTooLarge] = useState(false);
  const send = (file: File) => {
    setUnread(false);
    setTooLarge(file.size > ownThemeUploadMost);
    if (file.size > ownThemeUploadMost) {
      return;
    }
    file.text().then(
      (text) => {
        upload.mutate(
          { params: { query: { kind } }, ...asFileText(text) },
          {
            onSuccess: (made: Schema<"ThemeDetailBody">) => {
              void navigate({ to: themeEditorAddress(made.theme.kind, made.theme.id) });
            },
          },
        );
      },
      () => {
        setUnread(true);
      },
    );
  };
  return { upload, unread, tooLarge, send };
}

export function UploadTheme({ kind }: { readonly kind: Schema<"ThemeKind"> }) {
  const { upload, unread, tooLarge, send } = useUpload(kind);
  return (
    <>
      <FilePicker
        chooseLabel={t("theme-editor.upload")}
        onFiles={(files) => {
          const [file] = files;
          if (file !== undefined) {
            send(file);
          }
        }}
      />
      {unread ? <Alert tone={alertTone.danger}>{t("theme-editor.upload-unread")}</Alert> : null}
      {tooLarge ? (
        <Alert tone={alertTone.danger}>
          {t("theme-editor.upload-too-large", {
            most: Math.floor(ownThemeUploadMost / BYTES_IN_KB),
          })}
        </Alert>
      ) : null}
      {upload.isError ? (
        <Alert tone={alertTone.danger}>
          {[messageOf(upload.error), ...refusedValuesOf(upload.error)].join(" ")}
        </Alert>
      ) : null}
    </>
  );
}
