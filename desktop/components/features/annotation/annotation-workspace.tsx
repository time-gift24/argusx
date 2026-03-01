import { RightAnnotationPanel } from "./right-annotation-panel";
import { mockReviewData } from "./mock-review-data";

export function AnnotationWorkspace() {
  return (
    <div className="grid min-h-[70vh] grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_360px]">
      <section
        data-testid="review-left-pane"
        className="rounded-md border bg-background p-4"
      >
        <h2 className="text-sm font-semibold">待审核内容</h2>
        <p className="mt-2 text-sm text-muted-foreground">{mockReviewData.basicInfo.case_title}</p>
        <p className="mt-2 text-sm text-muted-foreground">{mockReviewData.basicInfo.case_summary}</p>
      </section>

      <RightAnnotationPanel />
    </div>
  );
}
