import { t } from "@bugaboo/messages";
import { Button, SaveStatus, partOf, part, type SaveState } from "@bugaboo/ui";

import { doNothing } from "../../shared/do-nothing.ts";
import { formatTime } from "../../shared/format.ts";
import { useShortcuts } from "../../shared/use-shortcuts.ts";
import type { Description } from "./use-description.ts";

/** Where the save stands, in the three words (UX §6). */
function saveStateOf(description: Description): SaveState {
  const { phase, savedAt } = description;
  if (phase.kind === "saving") {
    return "saving";
  }
  if (phase.kind === "lost") {
    return "not-saved";
  }
  return phase.kind === "reading" && savedAt !== null ? "saved" : "idle";
}

export interface DescriptionHeadProps {
  readonly description: Description;
  readonly titleId: string;
  readonly canChange: boolean;
}

/** The heading of the description with *Edit* and the three save words (*Saving…*, *Saved · 10:42*, *Not saved yet*). */
export function DescriptionHead({ description, titleId, canChange }: DescriptionHeadProps) {
  const { savedAt } = description;
  useShortcuts({ "edit-description": canChange ? description.open : doNothing });
  return (
    <div {...partOf(part.issueSectionHead)}>
      <h2 id={titleId} {...partOf(part.issueSectionTitle)}>
        {t("issue-field.description")}
      </h2>
      {canChange && description.phase.kind === "reading" ? (
        <Button onPress={description.open}>{t("issues.description.edit")}</Button>
      ) : null}
      <SaveStatus
        state={saveStateOf(description)}
        labels={{
          saving: t("save.saving"),
          saved:
            savedAt === null
              ? t("save.saved")
              : t("issues.saved-at", { time: formatTime(savedAt) }),
          "not-saved": t("save.not-saved"),
        }}
      />
    </div>
  );
}
