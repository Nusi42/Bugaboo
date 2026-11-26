import { mapIssuesMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { pixelsOf } from "@bugaboo/ui";
import { useState } from "react";

import type { MapSizes } from "./geometry.ts";
import type { Pointing } from "./MapCardButton.tsx";
import { fittingZoom, fullSize, type MapForm } from "./view-state.ts";

/** The sizes of the picture as the theme file gives them, at the size of the text the person chose. */
export function mapSizes(): MapSizes {
  return {
    cardWidth: pixelsOf("size-map-card-width"),
    cardHeight: pixelsOf("size-map-card-height"),
    gap: pixelsOf("size-map-gap"),
    rowGap: pixelsOf("size-map-row-gap"),
    head: pixelsOf("size-map-head"),
    arrow: pixelsOf("space-2"),
  };
}

/** What the map shows and how, and how the person changes it. */
export interface MapView {
  readonly form: MapForm;
  readonly setForm: (form: MapForm) => void;
  /** 1 is the size of the text. */
  readonly zoom: number;
  readonly setZoom: (zoom: number) => void;
  /** How often *Fit* was pressed. */
  readonly fits: number;
  readonly onFitPress: () => void;
  /** Takes the zoom at which the picture fits, as the largest zoom offered below it. */
  readonly onFit: (ratio: number) => void;
  readonly pointed: string | undefined;
  readonly chosen: string | undefined;
  readonly pointing: Pointing;
  readonly unchoose: () => void;
}

/** What the map shows and how: its form, its zoom, and the issue pointed at and chosen. */
export function useMapView(phone: boolean): MapView {
  const [form, setForm] = useState<MapForm>(phone ? "list" : "picture");
  const [zoom, setZoom] = useState(fullSize);
  const [fits, setFits] = useState(0);
  const [pointed, setPointed] = useState<string | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  return {
    form,
    setForm,
    zoom,
    setZoom,
    fits,
    onFitPress: () => {
      setFits((count) => count + 1);
    },
    onFit: (ratio: number) => {
      setZoom(fittingZoom(ratio));
    },
    pointed: pointed ?? undefined,
    chosen: chosen ?? undefined,
    pointing: {
      onPoint: setPointed,
      onLeave: () => {
        setPointed(null);
      },
      onChoose: (id: string) => {
        setChosen((was) => (was === id ? null : id));
      },
    },
    unchoose: () => {
      setChosen(null);
    },
  };
}

/** What a map says when it was cut: more issues are connected, or found, than it holds. */
export const cutWords = {
  issue: () => t("relation-map.too-many", { most: mapIssuesMost }),
  search: () => t("relation-map.too-many-found", { most: mapIssuesMost }),
} as const;
