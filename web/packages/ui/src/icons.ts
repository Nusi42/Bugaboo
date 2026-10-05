import {
  Activity,
  Building2,
  ChevronLeft,
  ChevronRight,
  Ellipsis,
  House,
  Keyboard,
  LayoutPanelLeft,
  LogOut,
  Menu,
  MonitorSmartphone,
  PanelLeftClose,
  PanelLeftOpen,
  Palette,
  Search,
  Settings,
  ShieldCheck,
  Undo2,
  UserPlus,
  X,
  type LucideIcon,
} from "lucide-react";

/**
 * The names of the icons that are used, to pass as `name` (no text literals in components). Only
 * the icons of this list are imported, so only those ship (UX §7.5).
 */
export const iconName = {
  back: "back",
  close: "close",
  frames: "frames",
  home: "home",
  invitations: "invitations",
  keyboard: "keyboard",
  menu: "menu",
  more: "more",
  narrow: "narrow",
  next: "next",
  organization: "organization",
  palette: "palette",
  search: "search",
  security: "security",
  sessions: "sessions",
  settings: "settings",
  "sign-out": "sign-out",
  status: "status",
  undo: "undo",
  widen: "widen",
} as const;

export type IconName = (typeof iconName)[keyof typeof iconName];

/** The drawing of each icon; the compiler makes sure no name is left out. */
export const glyphs: Readonly<Record<IconName, LucideIcon>> = {
  back: ChevronLeft,
  close: X,
  frames: LayoutPanelLeft,
  home: House,
  invitations: UserPlus,
  keyboard: Keyboard,
  menu: Menu,
  more: Ellipsis,
  narrow: PanelLeftClose,
  next: ChevronRight,
  organization: Building2,
  palette: Palette,
  search: Search,
  security: ShieldCheck,
  sessions: MonitorSmartphone,
  settings: Settings,
  "sign-out": LogOut,
  status: Activity,
  undo: Undo2,
  widen: PanelLeftOpen,
};
