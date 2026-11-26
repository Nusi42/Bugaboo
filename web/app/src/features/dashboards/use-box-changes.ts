import { t } from "@bugaboo/messages";
import { useState } from "react";

import { doNothing } from "../../shared/do-nothing.ts";
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
 * Why a change was refused, in words: a stale version is said as such (the dashboard is read again); every other refusal
 * says what the server said, among them that a dashboard holds at most 20 boxes, the ones the maker cannot see now
 * counted too (`dashboard.too-many-boxes`, M10-F55), and a search that does not check.
 */
function refusalOf(error: unknown): string {
  return codeOf(error) === "edit.version-changed" ? t("dashboards.stale") : messageOf(error);
}

/** Offers *Undo* of a change that says what it did; the *Undo* itself says nothing. */
function offerUndo(said: string | null, undo: () => void): void {
  if (said !== null) {
    undoStore.offer({ message: said, undo });
  }
}

/** After a change of what the boxes are, what each box answers is read again; a change of places reads nothing. */
function readBoxesAgain(dashboard: Dashboard): () => void {
  return () => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.dashboards.boxesOf(dashboard.id) });
  };
}

/**
 * The changes of the boxes of a dashboard (CFG-03): each is saved on the version the person looked at, offers *Undo*,
 * and a change of places shows at once, so the boxes stand where they were let go while it is saved. A refusal reads
 * the dashboard again and says why.
 */
export function useBoxChanges(dashboard: Dashboard) {
  const change = useChangeDashboard(dashboard);
  const [refused, setRefused] = useState<string | null>(null);
  const send = (
    boxes: readonly AskedBox[],
    version: number,
    said: string | null,
    afterSave: () => void = doNothing,
  ): void => {
    const before = dashboard.boxes.map((box) => bodyOf(box));
    setRefused(null);
    change.mutate(
      { boxes, version },
      {
        onSuccess: (changed: Dashboard) => {
          afterSave();
          offerUndo(said, () => {
            send(before, changed.version, null, afterSave);
          });
        },
        onError: (error: unknown) => {
          setRefused(refusalOf(error));
          refreshDashboards();
        },
      },
    );
  };
  return {
    refused,
    pending: change.isPending,
    // An added, changed or removed box: what the boxes answer is read again.
    save: ({ boxes, said }: BoxesChange) => {
      send(boxes, dashboard.version, said, readBoxesAgain(dashboard));
    },
    // New places of the boxes the person sees, in their order, shown at once.
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
