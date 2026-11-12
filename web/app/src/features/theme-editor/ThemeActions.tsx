import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, type ButtonLook } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { downloadTheme } from "./save-file.ts";
import { useCopyTheme } from "./themes.ts";
import { themeEditorAddress } from "./addresses.ts";

/** A button of a theme: the theme, and how the button looks where it stands (quiet in a bar). */
interface ActionProps {
  readonly theme: Schema<"ThemeBody">;
  readonly look: ButtonLook;
}

/** *Save a copy*: keeps the saved theme as a new one of the same kind, which opens in the editor. */
export function CopyThemeButton({ theme, look }: ActionProps) {
  const navigate = useNavigate();
  const copy = useCopyTheme();
  return (
    <>
      <Button
        look={look}
        isPending={copy.isPending}
        aria-label={t("theme-editor.copy-named", { name: theme.name })}
        onPress={() => {
          copy.mutate(
            {
              params: { path: { theme: theme.id } },
              body: { name: t("theme-editor.copy-name", { name: theme.name }), kind: theme.kind },
            },
            {
              onSuccess: (made: Schema<"ThemeDetailBody">) => {
                void navigate({ to: themeEditorAddress(made.theme.kind, made.theme.id) });
              },
            },
          );
        }}
      >
        {t("theme-editor.copy")}
      </Button>
      {copy.isError ? <Alert tone={alertTone.danger}>{messageOf(copy.error)}</Alert> : null}
    </>
  );
}

/** *Download*: the saved theme as a file that *Upload* takes again. */
export function DownloadThemeButton({ theme, look }: ActionProps) {
  const [failed, setFailed] = useState<unknown>(null);
  return (
    <>
      <Button
        look={look}
        aria-label={t("theme-editor.download-named", { name: theme.name })}
        onPress={() => {
          setFailed(null);
          downloadTheme(theme.id).catch(setFailed);
        }}
      >
        {t("theme-editor.download")}
      </Button>
      {failed === null ? null : <Alert tone={alertTone.danger}>{messageOf(failed)}</Alert>}
    </>
  );
}

/**
 * *Save a copy* and *Download* of a theme that was saved, side by side in the row they stand in (the
 * foot of the editor, a row of a list); a theme never saved has nothing to copy yet.
 */
export function ThemeActions({
  theme,
  look = buttonLook.quiet,
}: {
  readonly theme: Schema<"ThemeBody">;
  readonly look?: ButtonLook;
}) {
  return theme.saved ? (
    <>
      <CopyThemeButton theme={theme} look={look} />
      <DownloadThemeButton theme={theme} look={look} />
    </>
  ) : null;
}
