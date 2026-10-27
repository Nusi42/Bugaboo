import { presentationOf, type ListView } from "../../shared/list-view.ts";
import { withProject } from "../../shared/with-filter.ts";
import { SaveSearchButton, useSearchLanguage } from "../search/index.ts";

export interface ProjectSaveProps {
  /** The text of the search, without the project: a project's list has it filled in. */
  readonly text: string;
  readonly shortName: string;
  readonly view: ListView;
  readonly hasMistake: boolean;
}

/** *Save search* on a project's list: the saved search keeps the project in its text, and the columns with it. */
export function ProjectSave({ text, shortName, view, hasMistake }: ProjectSaveProps) {
  const language = useSearchLanguage();
  if (language === undefined) {
    return null;
  }
  return (
    <SaveSearchButton
      text={withProject(language, text, shortName)}
      presentation={presentationOf(view)}
      hasMistake={hasMistake}
    />
  );
}
