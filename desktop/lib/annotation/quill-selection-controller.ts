export type QuillSelectionRange = {
  index: number;
  length: number;
};

export function createQuillSelectionController({
  delayMs,
  onFire,
}: {
  delayMs: number;
  onFire: (range: QuillSelectionRange) => void;
}) {
  let timer: ReturnType<typeof setTimeout> | null = null;

  return {
    onSelectionChange(range: QuillSelectionRange | null) {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }

      if (!range || range.length <= 0) {
        return;
      }

      timer = setTimeout(() => {
        onFire(range);
      }, delayMs);
    },
    dispose() {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    },
  };
}
