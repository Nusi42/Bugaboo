// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/my-settings";

export { AppearancePage } from "./AppearancePage.tsx";
export { RegionPage } from "./RegionPage.tsx";
export { AssistantPage } from "./AssistantPage.tsx";
export { StartPage, type StartPageProps } from "./StartPage.tsx";
export { AccessKeysPage } from "./access-keys/AccessKeysPage.tsx";
export { HandlePage } from "./HandlePage.tsx";
export { ConnectedAppsPage } from "./connected-apps/ConnectedAppsPage.tsx";
// What the consent step of an AI app shares with the choices of an access key (one definition).
export { choiceName } from "./access-keys/choice.ts";
export { ProjectPicker as ProjectLimitPicker } from "./access-keys/Pickers.tsx";
// What agent keys share with access keys: the form parts, the key shown once and the day it runs out.
export { ShownOnce } from "./access-keys/ShownOnce.tsx";
export { useDraft as useKeyDraft, type Draft as KeyDraft } from "./access-keys/draft.ts";
export { Expiry as KeyExpiry } from "./access-keys/FormParts.tsx";
export { keyEndOf } from "./access-keys/expiry-day.ts";
export { rulesOptions as keyRulesOptions } from "./access-keys/access-keys.ts";
export { KeyFacts, LastUsed } from "./access-keys/KeyFacts.tsx";
