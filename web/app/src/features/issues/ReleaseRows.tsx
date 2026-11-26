import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { ReleasePicker, type ReleaseRef } from "../releases/index.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { readOnlyReason } from "./read-only.ts";
import { heldBy, releaseField, releaseFields, type ReleaseWord } from "./release-fields.ts";
import { rowStatus } from "./row-status.ts";

/** One line for each release, as high as a control; a release the issue keeps after it moved says so. */
function ReleaseLines({ releases }: { readonly releases: readonly ReleaseRef[] }) {
  return releases.map((release) => (
    <span key={release.id} {...partOf(part.startColumnLine)}>
      {release.offered ? release.name : t("releases.not-offered-here", { name: release.name })}
    </span>
  ));
}

/** The button that opens the picker: *Choose a release* while there is none, *Change* after that. */
export function ChangeRelease({
  name,
  empty,
  onPress,
}: {
  readonly name: string;
  readonly empty: boolean;
  readonly onPress: () => void;
}) {
  return (
    <Button
      look={buttonLook.quiet}
      aria-label={
        empty
          ? t("releases.choose-one-for", { field: name })
          : t("issues.change-field", { field: name })
      }
      onPress={onPress}
    >
      {empty ? t("releases.choose-one") : t("issues.change")}
    </Button>
  );
}

/**
 * *Found in release* and *Planned for release* (ISS-08, UX §4.4): the releases the issue names, and
 * the way to change them with the picker of releases. A row the layout does not show, or the person
 * may not see, is left out; one they may see and not change says why.
 */
function ReleaseRow({
  blocks,
  changes,
  word,
}: ChangeableRowsProps & { readonly word: ReleaseWord }) {
  const [picking, setPicking] = useState(false);
  const { issue, mask, form } = blocks;
  const held = heldBy(issue, word);
  const name = t(releaseFields[word].name);
  const canChange = mask.changes(word);
  if (!form.shows(word) || !mask.sees(word) || (!canChange && held.length === 0)) {
    return null;
  }
  return (
    <>
      <DetailRow
        name={name}
        reason={canChange ? undefined : readOnlyReason(name)}
        status={canChange ? rowStatus(changes, word) : undefined}
      >
        <div {...partOf(part.startColumn)}>
          <ReleaseLines releases={held} />
          {canChange ? (
            <ChangeRelease
              name={name}
              empty={held.length === 0}
              onPress={() => {
                setPicking(true);
              }}
            />
          ) : null}
        </div>
      </DetailRow>
      <ReleasePicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={issue.project?.short_name ?? ""}
        title={name}
        held={held}
        several={releaseFields[word].several}
        onChange={(releases) => {
          changes.change(releaseFields[word].values(releases));
        }}
      />
    </>
  );
}

/** *Found in release*: the releases the issue was found in. */
export function FoundInReleaseRow(props: ChangeableRowsProps) {
  return <ReleaseRow {...props} word={releaseField.found} />;
}

/** *Planned for release*: the release its fix is planned for. */
export function PlannedForReleaseRow(props: ChangeableRowsProps) {
  return <ReleaseRow {...props} word={releaseField.planned} />;
}
