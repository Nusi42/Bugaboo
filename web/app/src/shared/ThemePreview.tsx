import { pageAttributes, partOf, part, usePreviewSheet } from "@bugaboo/ui";

/** An own theme a sample shows: its ID and the address of its sheet, over the built-in theme it extends. */
export interface PreviewedOwn {
  readonly id: string;
  readonly sheet: string;
}

export interface ThemePreviewProps {
  /** The built-in theme to show, by its ID on the page (the `previewLook` attribute of the page frame). */
  readonly theme: string;
  /** The own theme laid over it, whose sheet the server makes (THM-04); none for a built-in theme. */
  readonly own?: PreviewedOwn | undefined;
}

/**
 * A small sample of a theme, drawn in that theme's own colors, so it can be chosen by sight. An own
 * theme's sample links the sheet of the saved theme while it is shown; the sheet sets the theme's
 * values on a sample that names it and nowhere else. It says nothing to a screen reader: the name
 * of the theme next to it does.
 */
export function ThemePreview({ theme, own }: ThemePreviewProps) {
  usePreviewSheet(own?.sheet);
  const attributes = {
    [pageAttributes.previewLook]: theme,
    ...(own === undefined ? {} : { [pageAttributes.ownTheme]: own.id }),
  };
  return (
    <span {...partOf(part.themePreview)} {...attributes} aria-hidden>
      <span {...partOf(part.themePreviewBar)} />
      <span {...partOf(part.themePreviewCard)}>
        <span {...partOf(part.themePreviewLine)} />
        <span {...partOf(part.themePreviewLine, { variants: ["short"] })} />
      </span>
    </span>
  );
}
