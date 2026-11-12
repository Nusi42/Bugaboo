import { t } from "@bugaboo/messages";
import { Icon, iconName, iconSize, partOf, part, type IconName } from "@bugaboo/ui";
import type { Rating } from "@bugaboo/wasm";

/** The variant of the part that draws a rating, by the rating (the theme file colors each). */
const looks = { easy: "easy", ok: "ok", "hard-to-read": "hard" } as const;

/** The sign that goes with a rating, so it is told by more than its color. */
const signs: Readonly<Record<Rating, IconName>> = {
  easy: iconName.check,
  ok: iconName.check,
  "hard-to-read": iconName.warning,
};

/** How readable a color is: a sign and the word, *Easy*, *OK* or *Hard to read* (the glossary's words). */
export function RatingWord({ rating }: { readonly rating: Rating }) {
  const words: Readonly<Record<Rating, string>> = {
    easy: t("theme-editor.rating.easy"),
    ok: t("theme-editor.rating.ok"),
    "hard-to-read": t("theme-editor.rating.hard"),
  };
  return (
    <span {...partOf(part.themeRating, { variants: [looks[rating]] })}>
      <Icon name={signs[rating]} size={iconSize.small} />
      {words[rating]}
    </span>
  );
}
