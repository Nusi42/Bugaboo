/**
 * The words of the ARIA standard for parts whose changes a screen reader says, not text for people: a problem said at
 * once (`alert`), news said when the person pauses (`status`), and a region that is not a status but whose changes are
 * said in a pause (`polite`). Written once; every live part takes them from here.
 */
export const live = { alert: "alert", status: "status", polite: "polite" } as const;
