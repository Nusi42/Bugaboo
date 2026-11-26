import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { ReleasePicker } from "../releases/index.ts";
import { ChangeRelease } from "./ReleaseRows.tsx";
import { heldBy, releaseFields, type ReleaseWord } from "./release-fields.ts";
import type { NewIssueValues } from "./new-issue-form.ts";

export interface NewIssueReleaseFieldProps {
  readonly word: ReleaseWord;
  readonly values: NewIssueValues;
  /** The project it is made in: its short name. */
  readonly project: string;
  readonly onChange: (values: NewIssueValues) => void;
}

/** *Found in release* or *Planned for release* on the form: the releases chosen so far, and the button that opens the picker. */
export function NewIssueReleaseField({
  word,
  values,
  project,
  onChange,
}: NewIssueReleaseFieldProps) {
  const [picking, setPicking] = useState(false);
  const field = releaseFields[word];
  const name = t(field.name);
  const held = heldBy(
    { found_in_release: values.foundInRelease, planned_for_release: values.plannedForRelease },
    word,
  );
  return (
    <div>
      <p {...partOf(part.fieldLabel)}>{name}</p>
      <div {...partOf(part.startColumn)}>
        {held.map((release) => (
          <span key={release.id}>{release.name}</span>
        ))}
        <ChangeRelease
          name={name}
          empty={held.length === 0}
          onPress={() => {
            setPicking(true);
          }}
        />
      </div>
      <ReleasePicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={project}
        title={name}
        held={held}
        several={field.several}
        onChange={(releases) => {
          onChange({ ...values, ...field.values(releases) });
        }}
      />
    </div>
  );
}
