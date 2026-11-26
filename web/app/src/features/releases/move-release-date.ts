import { t } from "@bugaboo/messages";

import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { setReleaseDate } from "./releases.ts";

/** What a screen knows of a release it moves: where it was made, its date and its version when it was read. */
interface Moving {
  readonly id: string;
  readonly name: string;
  readonly project: string;
  readonly release_date: string | null;
  readonly version: number;
}

/** Puts a release back to the *Release date* it had, on the version its change made. */
async function releaseBack(
  release: Moving,
  version: number,
  afterwards: () => Promise<void>,
): Promise<void> {
  try {
    await setReleaseDate(release, version, release.release_date);
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await afterwards();
}

/**
 * Gives a release another *Release date* from the version that was read, and offers *Undo*, which puts the old
 * date back on the version the change made: a change that somebody made after it comes back as a clash and is
 * not overwritten. The reason of a refusal is said where the offers stand. `afterwards` reads the screen again,
 * whatever came of it.
 */
export async function moveReleaseDate(
  release: Moving,
  day: string,
  afterwards: () => Promise<void>,
): Promise<void> {
  try {
    const made = await setReleaseDate(release, release.version, day);
    undoStore.offer({
      message: t("timeline.release-moved", { name: release.name }),
      ...(made === undefined
        ? {}
        : {
            undo: () => {
              void releaseBack(release, made, afterwards);
            },
          }),
    });
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await afterwards();
}
