import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";
import { CreateTheme } from "./CreateTheme.tsx";
import { ThemeRow } from "./ThemeRow.tsx";
import { useOwnThemes } from "./themes.ts";
import { UploadTheme } from "./UploadTheme.tsx";

/** Stands for the place in the introduction while its words are cut around it; never shown. */
const PLACE_MARK = "\u0001";

/** The introduction of *Themes for everyone*, with the place to choose them kept on one line. */
function everyoneIntro(): ReactNode {
  const [before, after] = t("theme-editor.list-everyone-intro", { place: PLACE_MARK }).split(
    PLACE_MARK,
  );
  return (
    <>
      {before}
      <span {...partOf(part.keptTogether)}>{t("theme-editor.appearance-place")}</span>
      {after}
    </>
  );
}

/** The words of a list of own themes, by kind. */
function listWords(kind: Schema<"ThemeKind">) {
  return kind === "for-everyone"
    ? {
        heading: t("theme-editor.list-everyone"),
        intro: everyoneIntro(),
        nothing: t("theme-editor.list-everyone-nothing"),
      }
    : {
        heading: t("theme-editor.list-personal"),
        intro: t("theme-editor.list-personal-intro"),
        nothing: t("theme-editor.list-personal-nothing"),
      };
}

/**
 * The own themes of one kind (UX §5.6, THM-04): *Themes for everyone* in *Look › Themes*, *My
 * themes* in *My settings › Appearance*, each with its facts and buttons, and *Create a theme* and
 * *Upload* above the list.
 */
export function OwnThemes({ kind }: { readonly kind: Schema<"ThemeKind"> }) {
  const list = useOwnThemes();
  const words = listWords(kind);
  const themes = (list.data?.themes ?? []).filter((theme) => theme.kind === kind);
  return (
    <Card heading={words.heading} intro={words.intro}>
      <Card heading={t("theme-editor.create-title")}>
        <CreateTheme kind={kind} />
        <UploadTheme kind={kind} />
      </Card>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {themes.length === 0 && list.isSuccess ? (
        <p {...partOf(part.muted)}>{words.nothing}</p>
      ) : null}
      <ul {...partOf(part.themeList)}>
        {themes.map((theme) => (
          <ThemeRow key={theme.id} theme={theme} />
        ))}
      </ul>
    </Card>
  );
}
