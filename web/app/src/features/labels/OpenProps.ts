/** Which item of the page is open in the side panel, and how to open and close one. */
export interface OpenProps {
  /** The ID of the group, label or tag that is open, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}
