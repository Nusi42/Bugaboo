// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/shell";

export { AppLayout } from "./AppLayout.tsx";
export { GuestLayout } from "./GuestLayout.tsx";
export { ErrorPage, NotFoundPage, UnknownAddressPage, PendingPage } from "./Pages.tsx";
export { QueryGate, type QueryGateProps } from "./QueryGate.tsx";
export {
  addQuickSearchSource,
  type QuickResult,
  type QuickSearchAsker,
  type QuickSearchSource,
} from "./quick-search-sources.ts";
export { addQuickAction, type QuickAction } from "./quick-search-actions.ts";
export { mayOpen } from "../../shared/places.ts";
export { useQuickResults } from "./use-quick-results.ts";
export { SettingsFrame, TopicList, type TopicListProps } from "./SettingsFrame.tsx";
export { NavLink, type NavLinkProps } from "./NavItem.tsx";
export { usePageTrail, type TrailStep } from "./page-trail.ts";
export {
  addSidebarSection,
  type SidebarSection,
  type SidebarSectionView,
} from "./sidebar-sections.ts";
export { ShortcutsPage } from "./ShortcutsPage.tsx";
export { FocusView, type FocusViewProps } from "./FocusView.tsx";
export { ListEditor, type ListEditorProps, type ListRow } from "./ListEditor.tsx";
export { SidePanel, type SidePanelProps } from "./SidePanel.tsx";
export { addOverlayHost, type OverlayHost } from "./overlay-hosts.ts";
export {
  closeOverlay,
  openOverlay,
  useOverlay,
  useOverlayOpen,
  type OverlayName,
} from "./overlay-state.ts";
export { useNarrowSidebarWhile } from "./sidebar-state.ts";
export { BarSlot } from "./bar-slot.ts";
export { useFocusViewOpen } from "./sidebar-state.ts";
