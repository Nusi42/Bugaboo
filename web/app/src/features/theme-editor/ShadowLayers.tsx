import { t } from "@bugaboo/messages";
import { Button, buttonLook, ownShadow, partOf, part, Switch } from "@bugaboo/ui";

import { CheckedField } from "./CheckedField.tsx";
import { ColorPick } from "./ColorPick.tsx";
import { wholeNumber, withShadow } from "./model.ts";
import type { ShadowLayer, ThemeValues } from "./themes.ts";

/** The parts of a layer that are numbers of pixels. */
type LayerNumber = "x" | "y" | "blur" | "spread";

/** The names of the parts of a layer that are numbers, in the order the fields stand in. */
function layerNumbers(): readonly { readonly key: LayerNumber; readonly label: string }[] {
  return [
    { key: "x", label: t("theme-editor.shadow-x") },
    { key: "y", label: t("theme-editor.shadow-y") },
    { key: "blur", label: t("theme-editor.shadow-blur") },
    { key: "spread", label: t("theme-editor.shadow-spread") },
  ];
}

/** What tells the layers apart on screen: their place, up to the most a shadow has (`ownShadow`). */
const LAYER_NAMES: readonly string[] = Array.from(
  { length: ownShadow.layers },
  (_, place) => `layer-${String(place + 1)}`,
);

/** The six digits of a color, which the picker of the device takes (it has no transparency). */
const RGB_LENGTH = 7;

/**
 * The layer a shadow of its own starts with: a thin, soft shade under the box, in `color`. The
 * person changes every part of it; the bounds are the base theme's (`ownShadow`).
 */
function firstLayer(color: string): ShadowLayer {
  return { x: 0, y: 1, blur: 2, spread: 0, color };
}

export interface ShadowLayersProps {
  readonly values: ThemeValues;
  /** The name of the built-in theme whose shadow counts while the theme has none of its own. */
  readonly startsFrom: string;
  /** The color a new layer starts with. */
  readonly shade: string;
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** The color of a layer: its sample, which is also the picker of the device, and the color typed. */
function LayerColor(props: {
  readonly layer: ShadowLayer;
  readonly make: (color: string) => ThemeValues;
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}) {
  const { layer, make, accepts, onChange } = props;
  return (
    <span {...partOf(part.themeColorFields)}>
      <ColorPick
        label={t("theme-editor.shadow-color")}
        value={layer.color.slice(0, RGB_LENGTH)}
        onPick={(text) => {
          if (accepts(make(text))) {
            onChange(make(text));
          }
        }}
      />
      <CheckedField
        label={t("theme-editor.shadow-color")}
        value={layer.color}
        make={make}
        accepts={accepts}
        onChange={onChange}
      />
    </span>
  );
}

/** One layer: its offsets, blur and spread in pixels and its color, each typed. */
function Layer(
  props: ShadowLayersProps & { readonly at: number; readonly layers: readonly ShadowLayer[] },
) {
  const { at, layers, values, accepts, onChange } = props;
  const layer = layers[at];
  if (layer === undefined) {
    return null;
  }
  const changed = (next: ShadowLayer) => withShadow(values, layers.with(at, next));
  return (
    <fieldset {...partOf(part.themeShadowLayer)}>
      <legend {...partOf(part.themeShadowLegend)}>
        {t("theme-editor.shadow-layer", { number: at + 1 })}
      </legend>
      {layerNumbers().map((number) => (
        <CheckedField
          key={number.key}
          label={number.label}
          unit={t("theme-editor.pixels")}
          value={String(layer[number.key])}
          make={(text) => {
            const pixels = wholeNumber(text);
            return pixels === null ? null : changed({ ...layer, [number.key]: pixels });
          }}
          accepts={accepts}
          onChange={onChange}
        />
      ))}
      <LayerColor
        layer={layer}
        make={(color) => changed({ ...layer, color })}
        accepts={accepts}
        onChange={onChange}
      />
      {layers.length > 1 ? (
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            onChange(withShadow(values, layers.toSpliced(at, 1)));
          }}
        >
          {t("theme-editor.shadow-remove")}
        </Button>
      ) : null}
    </fieldset>
  );
}

/** The shadow (UX §5.6, M8-Q10): the built-in theme's, or one of its own of at most two layers. */
export function ShadowLayers(props: ShadowLayersProps) {
  const { values, startsFrom, shade, onChange } = props;
  const layers = values.shadow ?? null;
  return (
    <>
      <Switch
        label={t("theme-editor.shadow-own", { theme: startsFrom })}
        isSelected={layers !== null}
        onChange={(on) => {
          onChange(withShadow(values, on ? [firstLayer(shade)] : null));
        }}
      />
      {(layers ?? []).map((_, at) => (
        <Layer key={LAYER_NAMES[at]} {...props} at={at} layers={layers ?? []} />
      ))}
      {layers !== null && layers.length < ownShadow.layers ? (
        <div {...partOf(part.row)}>
          <Button
            onPress={() => {
              onChange(withShadow(values, [...layers, firstLayer(shade)]));
            }}
          >
            {t("theme-editor.shadow-add")}
          </Button>
        </div>
      ) : null}
    </>
  );
}
