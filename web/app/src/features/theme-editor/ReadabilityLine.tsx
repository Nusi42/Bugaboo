import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import type { HardColor, ThemeRead } from "@bugaboo/wasm";
import { useState } from "react";

import { colorName } from "./model.ts";
import { READABILITY_ID } from "./focus-color.ts";

/** One color that is hard to read: on how many colors, *Show* to list them and *Change* to mend it. */
function HardColorRow({
  hard,
  onChange,
}: {
  readonly hard: HardColor;
  readonly onChange: (place: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const color = colorName(hard.place);
  return (
    <li {...partOf(part.themeHardPair)}>
      <span>{t("theme-editor.hard-color", { color, count: hard.with.length })}</span>
      <span {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          aria-label={t(open ? "theme-editor.hide-named" : "theme-editor.show-named", { color })}
          onPress={() => {
            setOpen(!open);
          }}
        >
          {t(open ? "theme-editor.hide" : "theme-editor.show")}
        </Button>
        <Button
          look={buttonLook.quiet}
          aria-label={t("theme-editor.change-named", { name: color })}
          onPress={() => {
            onChange(hard.place);
          }}
        >
          {t("theme-editor.change")}
        </Button>
      </span>
      {open ? (
        <ul {...partOf(part.themeHardWith)}>
          {hard.with.map((other) => (
            <li key={other}>{colorName(other)}</li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/**
 * The counts behind a verdict that nothing is hard to read, in plain words, shown on request. While the rating is
 * being loaded the button already stands in its place, switched off, so nothing under it moves when the rating comes.
 */
function Details({ read }: { readonly read: ThemeRead | null }) {
  const [open, setOpen] = useState(false);
  const summary = read?.summary;
  return (
    <div>
      <Button
        look={buttonLook.quiet}
        isDisabled={summary === undefined}
        onPress={() => {
          setOpen(!open);
        }}
      >
        {t(open ? "theme-editor.details-hide" : "theme-editor.details")}
      </Button>
      {open && summary !== undefined ? (
        <p {...partOf(part.muted)}>
          {t("theme-editor.readability-details", {
            total: summary.easy + summary.ok + summary.hard_to_read,
            easy: summary.easy,
            ok: summary.ok,
          })}
        </p>
      ) : null}
    </div>
  );
}

/**
 * The verdict of the readability in words. It counts the pairs that are hard to read (a text or a line
 * on a background, glossary *Readability*): a color hard to read on four backgrounds is four pairs.
 */
function verdictOf(read: ThemeRead | null, kind: Schema<"ThemeKind">): string {
  if (read === null) {
    return t("theme-editor.readability-loading");
  }
  const count = read.hard_colors.reduce((pairs, color) => pairs + color.with.length, 0);
  if (count === 0) {
    return t("theme-editor.readability-fine");
  }
  const verdict = t("theme-editor.readability-hard", { count });
  // What to fix is the colors, named under the verdict.
  return kind === "for-everyone"
    ? `${verdict} ${t("theme-editor.readability-fix", { count: read.hard_colors.length })}`
    : verdict;
}

export interface ReadabilityLineProps {
  /** What `bb-core` says about the theme; none while the rating is being loaded. */
  readonly read: ThemeRead | null;
  readonly kind: Schema<"ThemeKind">;
  /** A save of a theme for everyone was refused for it: the verdict is then the refusal. */
  readonly refused: boolean;
  /** Opens the color to change and puts the cursor in it. */
  readonly onChange: (place: string) => void;
}

/**
 * The readability on top of the editor (UX §5.6), led by its verdict, which stays on top on a phone
 * too: nothing is hard to read (the counts on request), or how many pairs are, each color with the
 * colors it is hard to read on and a way to change it (they scroll with the page), and for a theme
 * for everyone that they must be fixed to share it. A refused save brings the cursor to the verdict,
 * which then has the look of a refusal.
 */
export function ReadabilityLine({ read, kind, refused, onChange }: ReadabilityLineProps) {
  const hard = read?.hard_colors ?? [];
  return (
    <>
      {/* The band the verdict stands in stays on top while the groups scroll under it, and covers them up to the top of
          their box, so nothing scrolled shows above it or right under it. */}
      <div {...partOf(part.themeEditorVerdict)}>
        <output
          id={READABILITY_ID}
          tabIndex={-1}
          {...partOf(part.themeReadability, {
            states: [refused && "refused", !refused && hard.length > 0 && "hard"],
          })}
        >
          {verdictOf(read, kind)}
        </output>
      </div>
      {(read?.upside_down ?? []).map((wrong) => (
        <p key={[wrong.louder, wrong.quieter].join()} {...partOf(part.muted)}>
          {t("theme-editor.order-upside-down", {
            quieter: colorName(wrong.quieter),
            louder: colorName(wrong.louder),
          })}
        </p>
      ))}
      {hard.length === 0 ? <Details read={read} /> : null}
      {hard.length === 0 ? null : (
        <ul {...partOf(part.themeReadabilityList)}>
          {hard.map((color) => (
            <HardColorRow key={color.place} hard={color} onChange={onChange} />
          ))}
        </ul>
      )}
    </>
  );
}
