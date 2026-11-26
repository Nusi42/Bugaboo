import { t } from "@bugaboo/messages";
import { useState } from "react";

import { useMistakes, type FormMistakes } from "../../shared/mistakes.ts";
import type { Refused } from "../../shared/refused.ts";
import {
  changeRelease,
  createRelease,
  deleteRelease,
  type Changed,
  type Release,
  type ReleaseChange,
} from "./releases.ts";

/** What the form of a release is of: the project, the release to change (none makes one), and where to go after. */
export interface ReleaseFormInput {
  readonly project: { readonly id: string };
  readonly release: Release | undefined;
  readonly onClose: () => void;
}

/** What the form of a release gives the window: its boxes, what is sent, and the ways to go on. */
export interface ReleaseForm {
  readonly name: string;
  readonly setName: (name: string) => void;
  /** The *Release date* as the box holds it; empty for none. */
  readonly date: string;
  readonly setDate: (day: string) => void;
  /** Why the release goes back to *Planned*. */
  readonly reason: string;
  readonly setReason: (reason: string) => void;
  /** What the server refused, in words. */
  readonly problem: string | undefined;
  /** A change is on its way. */
  readonly busy: boolean;
  readonly mistakes: FormMistakes;
  readonly save: () => void;
  readonly withState: (state: NonNullable<ReleaseChange["state"]>) => void;
  readonly remove: () => void;
}

/** The values of the boxes of the form. */
function useBoxes(release: Release | undefined) {
  const [name, setName] = useState(release?.name ?? "");
  const [date, setDate] = useState(release?.releaseDate ?? "");
  const [reason, setReason] = useState("");
  return { name, setName, date, setDate, reason, setReason };
}

/** What a refusal says in the window: the server's reason, or that somebody changed the release first. */
function sayRefused(refused: Refused): string {
  return "reason" in refused ? refused.reason : t("releases.changed-first");
}

/** The refusal in the answer of a change, if it was one. */
function refusalOf(changed: Changed): Refused | null {
  return "refused" in changed ? changed.refused : null;
}

/** Sends a change and closes the window when the server took it; says why when it did not. */
function useSending(onClose: () => void) {
  const [problem, setProblem] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const run = async (work: () => Promise<Refused | null>): Promise<void> => {
    setBusy(true);
    setProblem(undefined);
    const refused = await work();
    setBusy(false);
    if (refused === null) {
      onClose();
    } else {
      setProblem(sayRefused(refused));
    }
  };
  return { problem, busy, run };
}

/** The state a release goes to, with the reason it goes back to *Planned* needs. */
function stateChange(state: NonNullable<ReleaseChange["state"]>, reason: string): ReleaseChange {
  // The server says whether a reason is needed (only after the release was out); the text is sent when there is one.
  const said = reason.trim();
  return { state, ...(state === "planned" && said !== "" ? { reason: said } : {}) };
}

/**
 * The form of a release: its boxes, what is sent, and the ways to go on (*Save*, *Mark as released*,
 * *Retire the release*, *Back to Planned*, *Delete*). The server judges every change.
 */
export function useReleaseForm({ project, release, onClose }: ReleaseFormInput): ReleaseForm {
  const boxes = useBoxes(release);
  const sending = useSending(onClose);
  const mistakes = useMistakes();
  const { name, date, reason } = boxes;
  const day = date === "" ? null : date;
  const change = (body: ReleaseChange): void => {
    if (release !== undefined) {
      void sending.run(async () => refusalOf(await changeRelease(release, body)));
    }
  };
  return {
    ...boxes,
    ...sending,
    mistakes,
    save: (): void => {
      if (release === undefined) {
        void sending.run(async () =>
          refusalOf(await createRelease(project.id, { name: name.trim(), release_date: day })),
        );
        return;
      }
      change({
        ...(name.trim() === release.name ? {} : { name: name.trim() }),
        ...(day === release.releaseDate ? {} : { release_date: day }),
      });
    },
    withState: (state: NonNullable<ReleaseChange["state"]>): void => {
      change(stateChange(state, reason));
    },
    remove: (): void => {
      if (release !== undefined) {
        void sending.run(() => deleteRelease(release));
      }
    },
  };
}
