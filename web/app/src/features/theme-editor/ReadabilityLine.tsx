import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import type { ReadabilitySummary } from "@bugaboo/wasm";

import { colorName, hardColorsOf, type NamedPair } from "./model.ts";

export interface HardPairsProps {
  readonly pairs: readonly NamedPair[];
  /** Opens the color to change to mend a pair, and puts the cursor in it. */
  readonly onShow: (place: string) => void;
}

/** The colors that are hard to read, each once with how many others and *Show*. */
export function HardPairs({ pairs, onShow }: HardPairsProps) {
  return (
    <ul {...partOf(part.themeHardPairs)}>
      {hardColorsOf(pairs).map((hard) => {
        const color = colorName(hard.place);
        return (
          <li key={hard.place} {...partOf(part.themeHardPair)}>
            <span>{t("theme-editor.hard-color", { color, count: hard.count })}</span>
            <Button
              look={buttonLook.quiet}
              aria-label={t("theme-editor.show-named", { color })}
              onPress={() => {
                onShow(hard.place);
              }}
            >
              {t("theme-editor.show")}
            </Button>
          </li>
        );
      })}
    </ul>
  );
}

export interface ReadabilityLineProps {
  /** The sums of `bb-core`; none while the rating is being loaded. */
  readonly summary: ReadabilitySummary | undefined;
  readonly kind: Schema<"ThemeKind">;
}

/**
 * The line on top of the editor that sums the readability up (UX §5.6): *42 pairs easy to read,
 * 1 hard to read*, and for a theme for everyone what has to happen before it can be saved. It
 * stays at the top on a phone too.
 */
export function ReadabilityLine({ summary, kind }: ReadabilityLineProps) {
  if (summary === undefined) {
    return (
      <output {...partOf(part.themeReadability)}>{t("theme-editor.readability-loading")}</output>
    );
  }
  const hard = summary.hard_to_read > 0;
  return (
    <output {...partOf(part.themeReadability, { states: [hard && "hard"] })}>
      {t("theme-editor.readability", {
        easy: summary.easy,
        ok: summary.ok,
        hard: summary.hard_to_read,
      })}
      {hard && kind === "for-everyone" ? ` ${t("theme-editor.readability-fix")}` : null}
    </output>
  );
}
