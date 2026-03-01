import { LeftReviewPane } from "./left-review-pane";
import { RightAnnotationPanel } from "./right-annotation-panel";

export function AnnotationWorkspace() {
  return (
    <div className="grid min-h-[70vh] grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_360px]">
      <LeftReviewPane />
      <RightAnnotationPanel />
    </div>
  );
}
