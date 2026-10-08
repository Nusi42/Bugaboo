import { partOf, part } from "@bugaboo/ui";

export interface ThemePreviewProps {
  /** The theme to show, by its ID on the page (`data-preview-look`). */
  readonly theme: string;
}

/**
 * A small sample of a theme, drawn in that theme's own colors, so it can be chosen by sight.
 * It says nothing to a screen reader: the name of the theme next to it does.
 */
export function ThemePreview({ theme }: ThemePreviewProps) {
  return (
    <span {...partOf(part.themePreview)} data-preview-look={theme} aria-hidden>
      <span {...partOf(part.themePreviewBar)} />
      <span {...partOf(part.themePreviewCard)}>
        <span {...partOf(part.themePreviewLine)} />
        <span {...partOf(part.themePreviewLine, { variants: ["short"] })} />
      </span>
    </span>
  );
}
