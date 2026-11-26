// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/search";

export { SavedSearchPage, type SavedSearchPageProps } from "./SavedSearchPage.tsx";
export { SearchPage, type SearchPageProps } from "./SearchPage.tsx";
export { IssuePicker } from "./IssueChoice.tsx";
export { registerSearch } from "./register.tsx";
export { SearchBar, type SearchBarProps } from "./SearchBar.tsx";
export { ViewChoices, type ListViewControl } from "./ViewChoices.tsx";
export { timelineGroupable, useSearchFields, type SearchFieldsAnswer } from "./search-fields.ts";
export { useSearchLanguage, useSearchLanguageRead } from "./search-language.ts";
export { useChangeSearch, useProjectListChoice } from "./saved-searches.ts";
export { SearchMailWindow } from "./SearchMailWindow.tsx";
export {
  useSearchMails,
  useSetSearchMail,
  useStopSearchMail,
  whenText,
  type SearchMail,
} from "./search-mails.ts";
export { SaveSearchButton, type SaveSearchButtonProps } from "./SaveSearchButton.tsx";
export { withSortBy } from "../../shared/sort-by.ts";
export {
  historyStep,
  readSearch,
  searchHref,
  searchLocation,
  validateSearch,
  type HistoryStep,
  type SearchLocation,
  type SearchState,
} from "../../shared/search-address.ts";
export { withGroupFilter } from "../../shared/with-filter.ts";
export { useCanonicalForms, type CanonicalForms } from "./canonical-forms.ts";
export { SavedSearchChoice, type SavedSearchChoiceProps } from "./SavedSearchChoice.tsx";
export {
  addSavedSearchAction,
  type SavedSearchAction,
  type SavedSearchActionProps,
} from "./saved-search-actions.ts";
