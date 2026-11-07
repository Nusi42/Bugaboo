// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/my-settings";

export { AppearancePage } from "./AppearancePage.tsx";
export { RegionPage } from "./RegionPage.tsx";
export { StartPage, type StartPageProps } from "./StartPage.tsx";
export { AccessKeysPage } from "./access-keys/AccessKeysPage.tsx";
export { HandlePage } from "./HandlePage.tsx";
export { ConnectedAppsPage } from "./connected-apps/ConnectedAppsPage.tsx";
// What the consent step of an AI app shares with the choices of an access key (one definition).
export { choiceName } from "./access-keys/choice.ts";
export { projectsOptions as projectChoicesOptions } from "./access-keys/access-keys.ts";
