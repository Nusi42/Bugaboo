import { t } from "@bugaboo/messages";
import {
  ownFontFaces,
  ownFonts,
  ownRadii,
  SelectField,
  type OwnFont,
  type OwnRadius,
} from "@bugaboo/ui";

import { CheckedField } from "./CheckedField.tsx";
import { wholeNumber, withFont, withRadius } from "./model.ts";
import type { ThemeValues } from "./themes.ts";

export interface CornersAndFacesProps {
  readonly values: ThemeValues;
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** The names of the corners. */
function radiusLabels(): Readonly<Record<OwnRadius, string>> {
  return {
    "r-ctl": t("theme-editor.corners-controls"),
    "r-card": t("theme-editor.corners-cards"),
  };
}

/** The names of the choices of text faces. */
function fontLabels(): Readonly<Record<OwnFont, string>> {
  return {
    "font-ui": t("theme-editor.font-text"),
    "font-display": t("theme-editor.font-headings"),
    "font-reader": t("theme-editor.font-long"),
  };
}

/**
 * The corners of controls and of cards in pixels, within the bounds of the base theme, and for
 * *Text*, *Headings* and *Long text* one of the built-in faces (UX §5.6, M8-Q10). No font files.
 */
export function CornersAndFaces({ values, accepts, onChange }: CornersAndFacesProps) {
  const labels = radiusLabels();
  const faces = fontLabels();
  return (
    <>
      {ownRadii.map((radius) => (
        <CheckedField
          key={radius.name}
          label={labels[radius.name]}
          description={t("theme-editor.corners-bounds", { min: radius.min, max: radius.max })}
          unit={t("theme-editor.pixels")}
          value={String(values.radii?.[radius.name] ?? radius.start)}
          make={(text) => {
            const pixels = wholeNumber(text);
            return pixels === null ? null : withRadius(values, radius.name, pixels);
          }}
          accepts={accepts}
          onChange={onChange}
        />
      ))}
      {ownFonts.map((font) => (
        <SelectField
          key={font}
          label={faces[font]}
          value={ownFonts.find((stack) => stack === values.fonts?.[font]) ?? font}
          choices={ownFonts.map((stack) => ({
            value: stack,
            label: t("theme-editor.face", { role: faces[stack], face: ownFontFaces[stack] }),
          }))}
          onChange={(stack) => {
            onChange(withFont(values, font, stack));
          }}
        />
      ))}
    </>
  );
}
