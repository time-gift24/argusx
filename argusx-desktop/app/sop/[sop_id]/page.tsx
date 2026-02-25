import SopClient from "./SopClient";

// Generate at least one static param to satisfy build requirements
// In production, this should be expanded to include all known SOP IDs
export async function generateStaticParams() {
  // Placeholder - in production, fetch all SOP IDs from backend
  return [
    { sop_id: "sample-sop" },
  ];
}

export default async function SopPage({ params }: { params: Promise<{ sop_id: string }> }) {
  const resolvedParams = await params;
  return <SopClient sopId={resolvedParams.sop_id} />;
}
