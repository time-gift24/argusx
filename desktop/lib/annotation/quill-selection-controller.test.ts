import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQuillSelectionController } from "@/lib/annotation/quill-selection-controller";

describe("quill selection controller", () => {
  beforeEach(() => vi.useFakeTimers());

  it("emits once after 300ms", () => {
    const onFire = vi.fn();
    const ctl = createQuillSelectionController({ delayMs: 300, onFire });

    ctl.onSelectionChange({ index: 5, length: 3 });
    vi.advanceTimersByTime(299);
    expect(onFire).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onFire).toHaveBeenCalledTimes(1);
  });

  it("cancels when selection collapses", () => {
    const onFire = vi.fn();
    const ctl = createQuillSelectionController({ delayMs: 300, onFire });

    ctl.onSelectionChange({ index: 5, length: 3 });
    ctl.onSelectionChange({ index: 5, length: 0 });
    vi.advanceTimersByTime(300);

    expect(onFire).not.toHaveBeenCalled();
  });
});
