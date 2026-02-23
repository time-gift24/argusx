import { Suspense } from "react";
import { SopDetailView } from "./sop-detail-view";

export function generateStaticParams() {
  return [{ id: "1" }];
}

export default function SopDetailPage({ params }: { params: Promise<{ id: string }> }) {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <SopDetailViewParams params={params} />
    </Suspense>
  );
}

async function SopDetailViewParams({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <SopDetailView id={id} />;
}
