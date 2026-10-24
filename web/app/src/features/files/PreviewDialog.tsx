import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, partOf, part } from "@bugaboo/ui";

import { DownloadButton } from "./FileLinks.tsx";
import type { FileItem } from "./file-item.ts";
import { FileName } from "./FileName.tsx";

/**
 * The preview of a file, larger (UX §4.4): a picture from our own origin, never the file itself,
 * so an SVG or an HTML page is never shown here. `url` is the signed link of the preview, read
 * just before; the dialog is closed while it is `null`.
 */
export function PreviewDialog({
  item,
  url,
  issue,
  onClose,
}: {
  readonly item: FileItem;
  readonly url: string | null;
  readonly issue: string;
  readonly onClose: () => void;
}) {
  return (
    <Overlay
      isOpen={url !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={t("files.preview-of", { name: item.name })}
    >
      <div {...partOf(part.stack)}>
        <FileName name={item.name} />
        {url === null ? null : (
          <img
            {...partOf(part.filesLarge)}
            src={url}
            alt={t("files.preview-of", { name: item.name })}
          />
        )}
        <div {...partOf(part.row)}>
          <DownloadButton item={item} issue={issue} />
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.close")}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}
