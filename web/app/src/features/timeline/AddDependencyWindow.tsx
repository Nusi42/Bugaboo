import { t } from "@bugaboo/messages";
import { ChoiceGroup, choiceGroupLook, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { windowForms } from "../../shared/window-forms.ts";
import { IssuePicker } from "../search/index.ts";
import type { Pair } from "./dependencies.ts";
import { TimelineWindow } from "./TimelineWindow.tsx";

/** Which of the two issues waits: the one of the bar, or the one that is chosen. */
type Way = "first" | "waits";

export interface AddDependencyWindowProps {
  /** The readable ID of the issue of the bar. */
  readonly readableId: string;
  readonly onAdd: (pair: Pair) => void;
  readonly onClose: () => void;
}

/**
 * *Add dependency…* (UX §4.7, TEST-A11Y): the typed way to drag the dot of a bar onto another bar.
 * The person says which issue waits and finds the other one with the usual issue finder.
 */
export function AddDependencyWindow({ readableId, onAdd, onClose }: AddDependencyWindowProps) {
  const [way, setWay] = useState<Way>("first");
  const [other, setOther] = useState<string>();
  const add = (): void => {
    if (other !== undefined) {
      onAdd(
        way === "first"
          ? { first: readableId, waiting: other }
          : { first: other, waiting: readableId },
      );
    }
  };
  return (
    <TimelineWindow
      form={windowForms.addDependency}
      title={t("timeline.add-dependency-title", { key: readableId })}
      submit={t("timeline.add-it")}
      onSubmit={add}
      onClose={onClose}
    >
      <ChoiceGroup
        legend={t("timeline.which-way")}
        look={choiceGroupLook.radios}
        choices={[
          { value: "first", label: t("timeline.must-finish-first", { key: readableId }) },
          { value: "waits", label: t("timeline.waits", { key: readableId }) },
        ]}
        value={way}
        onChange={setWay}
      />
      {other === undefined ? null : (
        <p {...partOf(part.text)}>{t("timeline.other-chosen", { key: other })}</p>
      )}
      <IssuePicker onPick={setOther} />
    </TimelineWindow>
  );
}
