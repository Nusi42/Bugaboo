// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/notifications";
import { lazyRouteComponent } from "@tanstack/react-router";

/** *Notifications* of the start page, loaded when the start page is first shown. */
export const NotificationsBlock = lazyRouteComponent(
  () => import("./NotificationsBlock.tsx"),
  "NotificationsBlock",
);

export { NotificationsPage } from "./NotificationsPage.tsx";
export { CustomizePage } from "./CustomizePage.tsx";
export { MailServerPage } from "./mail-server/MailServerPage.tsx";
export { MailServerPanel } from "./mail-server/MailServerPanel.tsx";
