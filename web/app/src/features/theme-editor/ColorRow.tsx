import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, TextField, type ColorToken } from "@bugaboo/ui";
import type { RatedPair, ShownColor } from "@bugaboo/wasm";
import { useRef } from "react";

import { colorWords } from "./color-words.ts";
import { colorRowId } from "./focus-color.ts";
import { ratingOf, withColor, withMore, withoutColor } from "./model.ts";
import { RatingWord } from "./RatingWord.tsx";
import type { ThemeValues } from "./themes.ts";
import { useTyped } from "./use-typed.ts";

/** Words of the HTML standard, not text for people. */
const colorInput = "color";

export interface ColorRowProps {
  readonly token: ColorToken;
  readonly values: ThemeValues;
  /** The color as the theme shows it now, with its links followed; none until the rating is there. */
  readonly shown: ShownColor | undefined;
  readonly pairs: readonly RatedPair[];
  /** The name of the built-in theme the own theme starts from: *Use Dusk's color*. */
  readonly startsFrom: string;
  /** Whether `bb-core` takes these values (a save would take them); false while it cannot tell. */
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** The sample of a color, which is also the picker of the device: a quicker way, never the only one. */
function ColorPick(props: {
  readonly label: string;
  readonly value: string;
  readonly onPick: (text: string) => void;
}) {
  const { label, value, onPick } = props;
  const field = useRef<HTMLInputElement>(null);
  return (
    <input
      ref={field}
      {...partOf(part.themeColorPick)}
      type={colorInput}
      aria-label={t("theme-editor.pick", { color: label })}
      value={value}
      onChange={() => {
        onPick(field.current?.value ?? value);
      }}
    />
  );
}

/** The color for a device that asks for more contrast, with how readable it is there. */
function MoreField({ token, values, shown, pairs, accepts, onChange }: ColorRowProps) {
  const more = useTyped(accepts, onChange);
  const own = values.colors?.[token];
  const stronger = ratingOf(pairs, token, true);
  return (
    <TextField
      label={t("theme-editor.more")}
      description={stronger === undefined ? undefined : <RatingWord rating={stronger} />}
      placeholder={t("theme-editor.more-same")}
      value={more.typed ?? own?.more ?? ""}
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
  const { token, values, shown, pairs, startsFrom, accepts, onChange } = props;
  const own = values.colors?.[token];
  const { label } = colorWords[token];
  const color = useTyped(accepts, onChange);
  const value = own?.value ?? shown?.value ?? "";
  const usual = ratingOf(pairs, token, false);
  const offer = (text: string) => {
    color.offer(text, withColor(values, token, text));
  };
  return (
    <div {...partOf(part.themeColor)} id={colorRowId(token)}>
      <div {...partOf(part.themeColorHead)}>
        <span>{label}</span>
        {usual === undefined ? null : <RatingWord rating={usual} />}
      </div>
      <div {...partOf(part.themeColorFields)}>
        <ColorPick label={label} value={value} onPick={offer} />
        <TextField
          label={t("theme-editor.color", { color: label })}
          hideLabel
          value={color.typed ?? value}
          errorMessage={color.typed === null ? undefined : t("theme-editor.not-a-color")}
          onChange={offer}
        />
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
