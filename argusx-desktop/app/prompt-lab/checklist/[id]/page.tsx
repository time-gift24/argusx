import { Suspense } from "react";
import { ChecklistDetailView } from "./checklist-detail-view";

export function generateStaticParams() {
  // Return a sample ID to allow Next.js to pre-render this page at build time
  return [{ id: "1" }];
}

export default function ChecklistDetailPage({ params }: { params: Promise<{ id: string }> }) {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <ChecklistDetailViewParams params={params} />
    </Suspense>
  );
}

async function ChecklistDetailViewParams({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <ChecklistDetailView id={id} />;
}
