import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useRef } from "react";

/** Words of the HTML standard, not text for people. */
const colorInput = "color";

export interface ColorPickProps {
  /** The name of the color, which the picker names. */
  readonly label: string;
  /** `#rrggbb`: the sample shows it. */
  readonly value: string;
  readonly onPick: (text: string) => void;
}

/**
 * The sample of a color, which is also the picker of the device: a quicker way to a color, never
 * the only one (every color can be typed beside it).
 */
export function ColorPick({ label, value, onPick }: ColorPickProps) {
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
