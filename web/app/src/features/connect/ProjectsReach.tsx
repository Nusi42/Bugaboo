import { t } from "@bugaboo/messages";
import { ChoiceGroup, partOf, part } from "@bugaboo/ui";

import { ProjectLimitPicker } from "../my-settings/index.ts";

/** Which projects the app may work in: every project the person can see, or only some of them. */
export type Reach = "all" | "some";

export interface ReachProps {
  readonly reach: Reach;
  readonly onReach: (reach: Reach) => void;
  readonly picked: readonly string[];
  readonly onPick: (project: string, on: boolean) => void;
}

/** *Projects*: all of them, or only the ones ticked; with none ticked, only some is not yet an answer. */
export function ProjectsReach({ reach, onReach, picked, onPick }: ReachProps) {
  return (
    <>
      <ChoiceGroup
        legend={t("connect.reach")}
        choices={[
          { value: "all", label: t("connect.reach-all") },
          { value: "some", label: t("connect.reach-some") },
        ]}
        value={reach}
        onChange={onReach}
      />
      {reach === "some" ? (
        <>
          <ProjectLimitPicker picked={picked} onPick={onPick} help={t("connect.reach-some-help")} />
          {picked.length === 0 ? (
            <p {...partOf(part.muted)}>{t("connect.reach-none-ticked")}</p>
          ) : null}
        </>
      ) : null}
    </>
  );
}
