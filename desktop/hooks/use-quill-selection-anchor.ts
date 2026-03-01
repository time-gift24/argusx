import * as React from "react";
import {
  createQuillSelectionController,
  type QuillSelectionRange,
} from "@/lib/annotation/quill-selection-controller";

export function useQuillSelectionAnchor({
  delayMs = 300,
  onFire,
}: {
  delayMs?: number;
  onFire: (range: QuillSelectionRange) => void;
}) {
  const controller = React.useMemo(
    () => createQuillSelectionController({ delayMs, onFire }),
    [delayMs, onFire],
  );

  React.useEffect(() => {
    return () => controller.dispose();
  }, [controller]);

  return controller.onSelectionChange;
}
