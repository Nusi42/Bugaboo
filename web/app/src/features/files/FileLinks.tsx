import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";
import { Button as PressableTile } from "react-aria-components";

import { useIssueFiles } from "./files.ts";
import { PreviewDialog } from "./PreviewDialog.tsx";
import { extensionOf } from "./size.ts";
import type { FileItem } from "./file-item.ts";

/** The link to use: the one in the list, read again, because a link is good for five minutes only. */
function useFreshLink(issue: string, id: string) {
  const list = useIssueFiles(issue);
  return async (which: "download_url" | "preview_url"): Promise<string | undefined> => {
    const fresh = await list.refetch();
    return fresh.data?.items.find((each) => each.id === id)?.[which] ?? undefined;
  };
}

/** The square at the start of a row: the preview, which opens larger when pressed, or the type. */
export function FileTile({ item, issue }: { readonly item: FileItem; readonly issue: string }) {
  const fresh = useFreshLink(issue, item.id);
  const [larger, setLarger] = useState<string | null>(null);
  if (item.preview_url === undefined || item.preview_url === null) {
    return (
      <span {...partOf(part.filesTile)} aria-hidden>
        {extensionOf(item.name)}
      </span>
    );
  }
  return (
    <>
      <PressableTile
        {...partOf(part.filesTile, { variants: ["picture"] })}
        aria-label={t("files.show-larger", { name: item.name })}
        onPress={() => {
          void fresh("preview_url").then((url) => {
            setLarger(url ?? null);
          });
        }}
      >
        <img
          {...partOf(part.filesPicture)}
          src={item.preview_url}
          alt={t("files.preview-of", { name: item.name })}
        />
      </PressableTile>
      <PreviewDialog
        item={item}
        url={larger}
        issue={issue}
        onClose={() => {
          setLarger(null);
        }}
      />
    </>
  );
}

/** *Download*: the link is read again just before, and the browser saves the file. */
export function DownloadButton({
  item,
  issue,
}: {
  readonly item: FileItem;
  readonly issue: string;
}) {
  const fresh = useFreshLink(issue, item.id);
  return (
    <Button
      look={buttonLook.quiet}
      aria-label={t("files.download-named", { name: item.name })}
      onPress={() => {
        void fresh("download_url").then((url) => {
          if (url !== undefined) {
            window.location.assign(url);
          }
        });
      }}
    >
      {t("files.download")}
    </Button>
  );
}
