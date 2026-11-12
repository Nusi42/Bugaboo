import { t } from "@bugaboo/messages";
import { ownFonts, ownRadii, SelectField, type OwnFont, type OwnRadius } from "@bugaboo/ui";

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

/** The names of the text faces, as the choice of one and as the stack it names. */
function fontWords(): Readonly<
  Record<OwnFont, { readonly label: string; readonly stack: string }>
> {
  return {
    "font-ui": { label: t("theme-editor.font-text"), stack: t("theme-editor.face-text") },
    "font-display": {
      label: t("theme-editor.font-headings"),
      stack: t("theme-editor.face-headings"),
    },
    "font-reader": { label: t("theme-editor.font-long"), stack: t("theme-editor.face-long") },
  };
}

/**
 * The corners of controls and of cards in pixels, within the bounds of the base theme, and for
 * *Text*, *Headings* and *Long text* one of the built-in faces (UX §5.6, M8-Q10). No font files.
 */
export function CornersAndFaces({ values, accepts, onChange }: CornersAndFacesProps) {
  const labels = radiusLabels();
  const faces = fontWords();
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
          label={faces[font].label}
          value={ownFonts.find((stack) => stack === values.fonts?.[font]) ?? font}
          choices={ownFonts.map((stack) => ({ value: stack, label: faces[stack].stack }))}
          onChange={(stack) => {
            onChange(withFont(values, font, stack));
          }}
        />
      ))}
    </>
  );
}
