import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, TextField, type ColorToken } from "@bugaboo/ui";
import type { ColorRating, ShownColor } from "@bugaboo/wasm";

import { ColorPick } from "./ColorPick.tsx";
import { colorWords } from "./color-words.ts";
import { colorRowId } from "./focus-color.ts";
import { ratingOf, withColor, withMore, withoutColor } from "./model.ts";
import { RatingWord } from "./RatingWord.tsx";
import type { ThemeValues } from "./themes.ts";
import { useTyped } from "./use-typed.ts";

export interface ColorRowProps {
  readonly token: ColorToken;
  readonly values: ThemeValues;
  /** The color as the theme shows it now, with its links followed; none until the rating is there. */
  readonly shown: ShownColor | undefined;
  /** The worst rating of each color, as `bb-core` worked it out. */
  readonly ratings: readonly ColorRating[];
  /** The name of the built-in theme the own theme starts from: *Use Dusk's color*. */
  readonly startsFrom: string;
  /** Whether `bb-core` takes these values (a save would take them); false while it cannot tell. */
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** The color for a device that asks for more contrast, with how readable it is there. */
function MoreField({ token, values, shown, ratings, accepts, onChange }: ColorRowProps) {
  const more = useTyped(accepts, onChange);
  const own = values.colors?.[token];
  const stronger = ratingOf(ratings, token, true);
  return (
    <TextField
      label={t("theme-editor.more")}
      description={stronger === undefined ? undefined : <RatingWord rating={stronger} />}
      value={more.typed ?? own?.more ?? shown?.more ?? ""}
      errorMessage={more.typed === null ? undefined : t("theme-editor.not-a-color")}
      onChange={(text) => {
        const kept = own?.value ?? shown?.value ?? "";
        more.offer(text, withMore(values, token, text === "" ? null : text, kept));
      }}
    />
  );
}

/**
 * One color of a group: its plain name and how readable it is, the color typed as `#` and six
 * digits (or picked from the device's own picker, which is its sample too), the color for a device
 * that asks for more contrast, and *Use Dusk's color* (UX §5.6). Every color can be typed; nothing
 * needs a drag.
 */
export function ColorRow(props: ColorRowProps) {
  const { token, values, shown, ratings, startsFrom, accepts, onChange } = props;
  const own = values.colors?.[token];
  const { label } = colorWords[token];
  const color = useTyped(accepts, onChange);
  const value = own?.value ?? shown?.value ?? "";
  const usual = ratingOf(ratings, token, false);
  const offer = (text: string) => {
    color.offer(text, withColor(values, token, text, shown?.built_in_more ?? undefined));
  };
  return (
    <div {...partOf(part.themeColor)} id={colorRowId(token)}>
      <div {...partOf(part.themeColorHead)}>
        <span>{label}</span>
        {usual === undefined ? null : <RatingWord rating={usual} />}
      </div>
      <div {...partOf(part.themeColorFields)}>
        <span {...partOf(part.themeColorPair)}>
          <ColorPick label={label} value={value} onPick={offer} />
          <TextField
            label={t("theme-editor.color", { color: label })}
            hideLabel
            value={color.typed ?? value}
            errorMessage={color.typed === null ? undefined : t("theme-editor.not-a-color")}
            onChange={offer}
          />
        </span>
        {own === undefined ? null : (
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              color.forget();
              onChange(withoutColor(values, token));
            }}
          >
            {t("theme-editor.use-built-in", { theme: startsFrom })}
          </Button>
        )}
      </div>
      <div {...partOf(part.themeColorFields)}>
        <MoreField {...props} key={own === undefined ? "built-in" : "own"} />
      </div>
    </div>
  );
}
