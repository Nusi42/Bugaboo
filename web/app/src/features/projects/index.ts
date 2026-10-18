// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/projects";

export { ProjectOverviewPage } from "./ProjectOverviewPage.tsx";
export { ProjectRolesPage } from "./ProjectRolesPage.tsx";
export { ProjectSettingsPage } from "./ProjectSettingsPage.tsx";
export { ProjectsPage, type ProjectsPageProps } from "./ProjectsPage.tsx";
export { registerProjects } from "./register.tsx";
export { ProjectGate, type ProjectGateProps } from "./ProjectGate.tsx";
export { ProjectTabs } from "./ProjectTabs.tsx";
export type { OneProject } from "./use-projects.ts";
export { linesOf, useProject, useProjectList, type ProjectLine } from "./use-projects.ts";
export { OfferedHere, type OfferedHereProps } from "./OfferedHere.tsx";
export { offerChangeBody } from "./offer-body.ts";
export { ProjectOffers, type ProjectOffer, type ProjectOffersProps } from "./ProjectOffers.tsx";
export {
  ProjectSettingsScreen,
  type ProjectSettingsScreenProps,
} from "./ProjectSettingsScreen.tsx";
export {
  OfferedIn,
  type OfferChangeLine,
  type OfferedInProps,
  type OfferLine,
} from "./OfferedIn.tsx";
export { PlaceEditorCard, type PlaceEditorCardProps } from "./PlaceEditorCard.tsx";
export { placeInherits, type EditedPlace } from "./place-inherits.ts";
