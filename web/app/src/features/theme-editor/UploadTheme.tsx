import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, FilePicker } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { refusedValuesOf } from "./refusals.ts";
import { themeFileIn, useUploadTheme } from "./themes.ts";
import { themeEditorAddress } from "./addresses.ts";

/**
 * *Upload*: makes an own theme of `kind` from a file a *Download* gave. The browser only reads the
 * file as JSON; the server judges its size and every value and names each one it refuses.
 */
export function UploadTheme({ kind }: { readonly kind: Schema<"ThemeKind"> }) {
  const navigate = useNavigate();
  const upload = useUploadTheme();
  const [unread, setUnread] = useState(false);
  return (
    <>
      <FilePicker
        chooseLabel={t("theme-editor.upload")}
        onFiles={(files) => {
          const file = files[0];
          if (file === undefined) {
            return;
          }
          setUnread(false);
          file.text().then(
            (text) => {
              const body = themeFileIn(text);
              if (body === null) {
                setUnread(true);
                return;
              }
              upload.mutate(
                { params: { query: { kind } }, body },
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
        }}
      />
      {unread ? <Alert tone={alertTone.danger}>{t("theme-editor.upload-unread")}</Alert> : null}
      {upload.isError ? (
        <Alert tone={alertTone.danger}>
          {[messageOf(upload.error), ...refusedValuesOf(upload.error)].join(" ")}
        </Alert>
      ) : null}
    </>
  );
}
