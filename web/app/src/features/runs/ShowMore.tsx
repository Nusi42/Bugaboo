import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";

/** *Show more*: reads the next page of changes of a run; the one button of every list of changes. */
export function ShowMore({
  isLoading,
  onMore,
}: {
  readonly isLoading: boolean;
  readonly onMore: () => void;
}) {
  return (
    <div>
      <Button isPending={isLoading} onPress={onMore}>
        {t("runs.show-more")}
      </Button>
    </div>
  );
}
