// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/settings";

export {
  ProjectSettingsSection,
  type ProjectSettingsSectionProps,
} from "./ProjectSettingsSection.tsx";
export { type SettingActions } from "./setting-actions.ts";
export { type ControlRenderer } from "./SettingRow.tsx";
export { SettingsForm, type SettingsFormProps } from "./SettingsForm.tsx";
export { SecurityPage } from "./SecurityPage.tsx";
export { SignInRulesPage } from "./SignInRulesPage.tsx";
export { ThemesPage } from "./ThemesPage.tsx";
export { sourceInWords, wordsOf } from "./texts.ts";
export { useOrganizationSettings } from "./organization.ts";
