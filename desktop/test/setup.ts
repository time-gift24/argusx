import "@testing-library/jest-dom";

if (typeof globalThis.IntersectionObserver === "undefined") {
  class MockIntersectionObserver implements IntersectionObserver {
    readonly root = null;
    readonly rootMargin = "";
    readonly thresholds = [0];

    constructor(
      private readonly callback: IntersectionObserverCallback
    ) {}

    disconnect() {}

    observe(target: Element) {
      this.callback(
        [{ isIntersecting: true, target } as IntersectionObserverEntry],
        this
      );
    }

    takeRecords() {
      return [{ isIntersecting: true } as IntersectionObserverEntry];
    }

    unobserve() {}
  }

  globalThis.IntersectionObserver = MockIntersectionObserver;
}
