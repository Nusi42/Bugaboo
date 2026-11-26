import { t, type MessageKey } from "@bugaboo/messages";

import { formatCalendarDay } from "../../shared/format.ts";
import type { Release, StateWord } from "./releases.ts";

/** The name of each state of a release (ADR-0049, decision 2). */
const stateName: Readonly<Record<StateWord, MessageKey>> = {
  planned: "release-state.planned",
  released: "release-state.released",
  retired: "release-state.retired",
};

/** *Planned*, *Released* or *Retired*, in words. */
export function stateText(state: StateWord): string {
  return t(stateName[state]);
}

/** The *Release date* of a release in the person's date format; a dash while it has none. */
export function releaseDateText(release: Pick<Release, "releaseDate">): string {
  return release.releaseDate === null
    ? t("releases.no-date")
    : formatCalendarDay(release.releaseDate);
}

/** *Released on* of a release in the person's date format; nothing while it has none. */
export function releasedOnText(release: Pick<Release, "releasedOn">): string {
  return release.releasedOn === null ? "" : formatCalendarDay(release.releasedOn);
}

/** The release in a sentence or a list: its name, and where it comes from when it is not made here. */
export function releaseNameText(release: Release, here: string | undefined): string {
  return here === undefined || release.project.short_name === here
    ? release.name
    : t("releases.from-project", { name: release.name, project: release.project.name });
}
