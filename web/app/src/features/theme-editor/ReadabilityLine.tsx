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

/** The counts behind a verdict that nothing is hard to read, in plain words, shown on request. */
function Details({ read }: { readonly read: ThemeRead }) {
  const [open, setOpen] = useState(false);
  const { easy, ok, hard_to_read: hard } = read.summary;
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          setOpen(!open);
        }}
      >
        {t(open ? "theme-editor.details-hide" : "theme-editor.details")}
      </Button>
      {open ? (
        <p {...partOf(part.muted)}>
          {t("theme-editor.readability-details", { total: easy + ok + hard, easy, ok })}
        </p>
      ) : null}
    </>
  );
}

/** The verdict of the readability in words. */
function verdictOf(read: ThemeRead | null, kind: Schema<"ThemeKind">): string {
  if (read === null) {
    return t("theme-editor.readability-loading");
  }
  const count = read.hard_colors.length;
  if (count === 0) {
    return t("theme-editor.readability-fine");
  }
  const verdict = t("theme-editor.readability-hard", { count });
  return kind === "for-everyone"
    ? `${verdict} ${t("theme-editor.readability-fix", { count })}`
    : verdict;
}

export interface ReadabilityLineProps {
  /** What `bb-core` says about the theme; none while the rating is being loaded. */
  readonly read: ThemeRead | null;
  readonly kind: Schema<"ThemeKind">;
  /** Opens the color to change and puts the cursor in it. */
  readonly onChange: (place: string) => void;
}

/**
 * The readability on top of the editor (UX §5.6), led by its verdict: nothing is hard to read (the
 * counts on request), or how many colors are, each with the colors it is hard to read on and a way
 * to change it, and for a theme for everyone that they must be fixed to share it. It stays at the
 * top on a phone too, and a refused save brings the cursor here.
 */
export function ReadabilityLine({ read, kind, onChange }: ReadabilityLineProps) {
  const hard = read?.hard_colors ?? [];
  return (
    <section
      id={READABILITY_ID}
      tabIndex={-1}
      aria-label={t("theme-editor.readability-title")}
      {...partOf(part.themeReadability, { states: [hard.length > 0 && "hard"] })}
    >
      <output>{verdictOf(read, kind)}</output>
      {read !== null && hard.length === 0 ? <Details read={read} /> : null}
      {hard.length === 0 ? null : (
        <ul {...partOf(part.themeHardPairs)}>
          {hard.map((color) => (
            <HardColorRow key={color.place} hard={color} onChange={onChange} />
          ))}
        </ul>
      )}
    </section>
  );
}
