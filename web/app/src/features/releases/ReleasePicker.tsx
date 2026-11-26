import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Overlay,
  PickerHint,
  PickerRow,
  SearchPicker,
  part,
  partOf,
} from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePickerFrame } from "../../shared/picker-status.ts";
import { releaseDateText, releaseNameText } from "./release-words.ts";
import { offeredFor, refOf, useReleases, type Release, type ReleaseRef } from "./releases.ts";

export interface ReleasePickerProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** The project of the issue (an ID or a short name): the releases offered there are chosen from. */
  readonly project: string;
  /** What the field is called: the title of the picker. */
  readonly title: string;
  /** The releases the issue names now. */
  readonly held: readonly ReleaseRef[];
  /** *Planned for release* holds one release, *Found in release* several. */
  readonly several: boolean;
  /** The releases the issue names from now on: all of them. */
  readonly onChange: (releases: readonly ReleaseRef[]) => void;
}

interface RowProps {
  readonly release: Release;
  readonly project: string;
  readonly chosen: boolean;
  readonly onChoose: () => void;
  readonly onTakeOut: () => void;
}

/** A release that is offered: a button to choose it, or the word that it is chosen and a button to take it out. */
function Row({ release, project, chosen, onChoose, onTakeOut }: RowProps) {
  const name = releaseNameText(release, project);
  return (
    <PickerRow label={name}>
      <PickerHint>{releaseDateText(release)}</PickerHint>
      {chosen ? (
        <Button aria-label={t("releases.take-out-name", { name })} onPress={onTakeOut}>
          {t("releases.take-out")}
        </Button>
      ) : (
        <Button aria-label={t("releases.choose-name", { name })} onPress={onChoose}>
          {t("releases.choose")}
        </Button>
      )}
    </PickerRow>
  );
}

/** Whether a release is found by the words typed: any of them, in its name. */
function matches(release: Release, words: string): boolean {
  const name = release.name.toLowerCase();
  return words
    .toLowerCase()
    .split(/\s+/u)
    .every((word) => name.includes(word));
}

/** The inside of the dialog, drawn only while it is open so nothing is asked before. */
function Inside({
  project,
  title,
  held,
  several,
  onChange,
  onClose,
}: Omit<ReleasePickerProps, "isOpen">) {
  const { words, frame } = usePickerFrame();
  const read = useReleases(project);
  const shown = offeredFor(read.releases, held).filter((release) => matches(release, words));
  const choose = (release: Release): void => {
    onChange(several ? [...held, refOf(release)] : [refOf(release)]);
    if (!several) {
      onClose();
    }
  };
  return (
    <div {...partOf(part.assignedPicker)}>
      <SearchPicker
        legend={title}
        searchLabel={t("releases.search")}
        listLabel={t("releases.list")}
        {...frame}
        failed={read.isError ? messageOf(read.error) : undefined}
        nothing={!read.isPending && shown.length === 0 ? t("releases.nothing") : undefined}
        more={undefined}
      >
        {shown.map((release) => (
          <Row
            key={release.id}
            release={release}
            project={project}
            chosen={held.some((one) => one.id === release.id)}
            onChoose={() => {
              choose(release);
            }}
            onTakeOut={() => {
              onChange(held.filter((one) => one.id !== release.id));
            }}
          />
        ))}
      </SearchPicker>
      <Button look={buttonLook.primary} onPress={onClose}>
        {t("common.done")}
      </Button>
    </div>
  );
}

/**
 * Choosing the releases an issue names (ISS-08, ADR-0049): the releases offered in its project, found
 * by name, each with a button to choose or take out; every press saves at once. The one picker of
 * *Found in release* and *Planned for release*, on the issue page and on the form of a new issue. A
 * retired release is offered only while the issue names it.
 */
export function ReleasePicker({ isOpen, onClose, ...inside }: ReleasePickerProps) {
  return (
    <Overlay
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={inside.title}
    >
      {isOpen ? <Inside {...inside} onClose={onClose} /> : null}
    </Overlay>
  );
}
