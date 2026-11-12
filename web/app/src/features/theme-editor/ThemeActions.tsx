import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, type ButtonLook } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { messageOf } from "../../shared/errors.ts";
import { downloadTheme } from "./save-file.ts";
import { useCopyTheme } from "./themes.ts";
import { themeEditorAddress } from "./addresses.ts";

/** A button of a theme: the theme, how the button looks where it stands, and where its failure goes. */
interface ActionProps {
  readonly theme: Schema<"ThemeBody">;
  readonly look: ButtonLook;
  /** Why the action did not work; the row shows it under its buttons, never among them. */
  readonly onFailed: (failure: unknown) => void;
}

/** *Save a copy*: keeps the saved theme as a new one of the same kind, which opens in the editor. */
function CopyThemeButton({ theme, look, onFailed }: ActionProps) {
  const navigate = useNavigate();
  const copy = useCopyTheme();
  return (
    <Button
      look={look}
      isPending={copy.isPending}
      aria-label={t("theme-editor.copy-named", { name: theme.name })}
      onPress={() => {
        onFailed(null);
        copy.mutate(
          {
            params: { path: { theme: theme.id } },
            body: { name: t("theme-editor.copy-name", { name: theme.name }), kind: theme.kind },
          },
          {
            onSuccess: (made: Schema<"ThemeDetailBody">) => {
              void navigate({ to: themeEditorAddress(made.theme.kind, made.theme.id) });
            },
            onError: onFailed,
          },
        );
      }}
    >
      {t("theme-editor.copy")}
    </Button>
  );
}

/** *Download*: the saved theme as a file that *Upload* takes again. */
function DownloadThemeButton({ theme, look, onFailed }: ActionProps) {
  return (
    <Button
      look={look}
      aria-label={t("theme-editor.download-named", { name: theme.name })}
      onPress={() => {
        onFailed(null);
        downloadTheme(theme.id).catch(onFailed);
      }}
    >
      {t("theme-editor.download")}
    </Button>
  );
}

/**
 * *Save a copy* and *Download* of a theme that was saved, side by side in the row they stand in (the
 * foot of the editor, a row of a list); a theme never saved has nothing to copy yet.
 */
export function ThemeActions({
  theme,
  look = buttonLook.quiet,
  onFailed,
}: Omit<ActionProps, "look"> & { readonly look?: ButtonLook }) {
  return theme.saved ? (
    <>
      <CopyThemeButton theme={theme} look={look} onFailed={onFailed} />
      <DownloadThemeButton theme={theme} look={look} onFailed={onFailed} />
    </>
  ) : null;
}

/** Why *Save a copy* or *Download* did not work, under the row of buttons; nothing when they did. */
export function ActionFailure({ failure }: { readonly failure: unknown }) {
  return failure === null || failure === undefined ? null : (
    <Alert tone={alertTone.danger}>{messageOf(failure)}</Alert>
  );
}
