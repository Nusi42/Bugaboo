// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/layouts";

export { FormNotice } from "./FormNotice.tsx";
export { FormSections, type FormSectionsProps } from "./FormSections.tsx";
export { OrganizationLayoutsScreen, ProjectLayoutsScreen } from "./LayoutsScreens.tsx";
export type { LayoutsScreenProps, ProjectLayoutsScreenProps } from "./LayoutsScreens.tsx";
export { isPlain, onlyFields } from "./form-tabs.ts";
export {
  formViewOf,
  useFormLayout,
  type FormKind,
  type FormSection,
  type FormView,
} from "./layouts.ts";
