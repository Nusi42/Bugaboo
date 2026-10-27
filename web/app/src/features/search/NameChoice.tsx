import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import { NameFinder, type NameFinderProps } from "./NameFinder.tsx";

export interface NameChoiceProps extends Omit<NameFinderProps, "onPick"> {
  /** The name chosen so far; none yet when empty. */
  readonly chosen: string;
  /** A name was chosen: the name itself, as the language reads it. */
  readonly onChoose: (name: string) => void;
}

/** The value of a filter on a person, a group or a project: the chosen name, and the finder. */
export function NameChoice({ language, what, chosen, onChoose }: NameChoiceProps) {
  return (
    <div {...partOf(part.searchList)}>
      {chosen === "" ? null : (
        <p {...partOf(part.muted)}>{t("search.bar.pick-chosen", { name: chosen })}</p>
      )}
      <NameFinder
        language={language}
        what={what}
        onPick={(name) => {
          onChoose(name.value);
        }}
      />
    </div>
  );
}
