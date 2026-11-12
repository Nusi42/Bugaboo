import type { Schema } from "@bugaboo/api-client";

/** An own theme as a theme picker offers it: what the setting keeps, its name and its sheet. */
export interface OwnThemeChoice {
  /** The value the setting keeps when the person chooses it. */
  readonly value: string;
  readonly label: string;
  /** The built-in theme it extends, which its sample is laid over. */
  readonly look: string;
  /** The ID of the theme and the address of its sheet, which draws its sample. */
  readonly own: { readonly id: string; readonly sheet: string };
}

/** The kinds of own themes, by name: for everyone in the organization, or personal to its maker. */
export const themeKinds = { forEveryone: "for-everyone", personal: "personal" } as const;

/** The order of the kinds in a picker: the organization's themes first, then the person's own. */
const kindOrder: readonly Schema<"ThemeKind">[] = [themeKinds.forEveryone, themeKinds.personal];

/**
 * The own themes a picker of a light or a dark theme offers after the built-in ones (ADR-0026,
 * update M8; UX §4.13): the saved themes of that look and of `kinds`, those for everyone first,
 * then the person's own. A theme never saved cannot be chosen.
 */
export function ownThemeChoices(
  themes: readonly Schema<"ThemeBody">[],
  light: boolean,
  kinds: readonly Schema<"ThemeKind">[],
): readonly OwnThemeChoice[] {
  return kindOrder
    .filter((kind) => kinds.includes(kind))
    .flatMap((kind) =>
      themes.flatMap((theme) =>
        theme.kind === kind &&
        theme.light === light &&
        theme.sheet !== undefined &&
        theme.sheet !== null
          ? [
              {
                value: theme.choice,
                label: theme.name,
                look: theme.extends,
                own: { id: theme.id, sheet: theme.sheet },
              },
            ]
          : [],
      ),
    );
}
