import type { SearchLanguage } from "@bugaboo/wasm-search";

import type { ResultsInput } from "../../shared/use-list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import type { IssueShow } from "./issue-show.ts";
import type { IssueSelection } from "./use-selection.ts";

/** What a list of the results of a search is given: what the page gives it, and where it stands. */
export interface IssueResultsProps extends ResultsInput {
  /** The ID of the project the search looks in; none looks in every project the person sees. */
  readonly within: string | undefined;
  /** The name of the project, for the count line; none for every project. */
  readonly scopeName: string | undefined;
  /** The short name of the project: where its list can make an issue and the bars act. */
  readonly project: string | undefined;
  readonly openIn: IssueShow;
  readonly open: string | undefined;
  readonly selection: IssueSelection;
}

/** The same once the search language and the fields the person may read are there. */
export interface LoadedProps extends IssueResultsProps {
  readonly language: SearchLanguage;
  readonly fields: SearchFieldsAnswer;
}
