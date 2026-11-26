import { boxesMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import {
  bodyOf,
  refreshDashboards,
  useChangeDashboard,
  type AskedBox,
  type Box,
  type Dashboard,
} from "./dashboards.ts";
import type { GridPlace } from "./grid-layout.ts";

/** What a change of the boxes is told: the boxes after it, and the sentence its *Undo* offer says. */
export interface BoxesChange {
  readonly boxes: readonly AskedBox[];
  readonly said: string;
}

/** The key of a dashboard in the cache: *Home* has its own. */
function keyOf(dashboard: Dashboard) {
  return dashboard.home ? queryKeys.dashboards.home() : queryKeys.dashboards.one(dashboard.id);
}

/**
 * Why a change was refused, in words: a stale version is said as such (the dashboard is read again), and a change the
 * server refused as too many boxes while the person sees fewer than the most says that boxes they cannot see now count
 * too (M10-F55).
 */
function refusalOf(error: unknown, dashboard: Dashboard): string {
  if (codeOf(error) === "edit.version-changed") {
    return t("dashboards.stale");
  }
  if (codeOf(error) === "request.invalid" && dashboard.boxes.length < boxesMost) {
    return t("dashboards.full-unseen", { most: boxesMost });
  }
  return messageOf(error);
}

/**
 * The changes of the boxes of a dashboard (CFG-03): each is saved on the version the person looked at, offers *Undo*,
 * and a change of places shows at once, so the boxes stand where they were let go while it is saved. A refusal reads
 * the dashboard again and says why.
 */
export function useBoxChanges(dashboard: Dashboard) {
  const change = useChangeDashboard(dashboard);
  const [refused, setRefused] = useState<string | null>(null);
  const send = (boxes: readonly AskedBox[], version: number, said: string | null): void => {
    const before = dashboard.boxes.map((box) => bodyOf(box));
    setRefused(null);
    change.mutate(
      { boxes, version },
      {
        onSuccess: (changed: Dashboard) => {
          if (said !== null) {
            undoStore.offer({
              message: said,
              undo: () => {
                send(before, changed.version, null);
              },
            });
          }
        },
        onError: (error: unknown) => {
          setRefused(refusalOf(error, dashboard));
          refreshDashboards();
        },
      },
    );
  };
  return {
    refused,
    pending: change.isPending,
    /** Saves the boxes after a change: an added, changed or removed box. */
    save: ({ boxes, said }: BoxesChange) => {
      send(boxes, dashboard.version, said);
    },
    /** Saves new places of the boxes the person sees, in their order, and shows them at once. */
    place: (places: readonly GridPlace[], said: string) => {
      const boxes = dashboard.boxes.map((box, at) => ({ ...box, ...places[at] }));
      queryClient.setQueryData(keyOf(dashboard), { ...dashboard, boxes });
      send(
        boxes.map((box: Box) => bodyOf(box)),
        dashboard.version,
        said,
      );
    },
  };
}
