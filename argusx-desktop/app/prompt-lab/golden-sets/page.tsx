"use client";

import { useState, useEffect } from "react";
import { Plus, GripVertical, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import {
  listGoldenSetItems,
  unbindGoldenSetItem,
  type GoldenSetItem,
} from "@/lib/api/prompt-lab";
import { PromptLabBreadcrumb } from "@/components/layouts/prompt-lab-breadcrumb";
// Mock golden set display
const goldenSets = [
  { id: 1, name: "Default Set", itemCount: 2 },
];

export default function GoldenSetsPage() {
  const [items, setItems] = useState<GoldenSetItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    listGoldenSetItems(1).then((data) => {
      setItems(data);
      setLoading(false);
    });
  }, []);

  const handleUnbind = async (goldenSetId: number, checklistItemId: number) => {
    await unbindGoldenSetItem(goldenSetId, checklistItemId);
    setItems(items.filter(
      (i) => !(i.golden_set_id === goldenSetId && i.checklist_item_id === checklistItemId)
    ));
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <PromptLabBreadcrumb />
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Golden Sets</h1>
        <Button>
          <Plus className="h-4 w-4 mr-2" />
          Create Set
        </Button>
      </div>

      {goldenSets.map((set) => (
        <Card key={set.id}>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              {set.name}
              <Badge>{set.itemCount} items</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2">
              {items
                .filter((i) => i.golden_set_id === set.id)
                .map((item) => (
                  <li
                    key={item.checklist_item_id}
                    className="flex items-center justify-between p-2 rounded-md bg-muted"
                  >
                    <div className="flex items-center gap-2">
                      <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />
                      <span>Checklist Item #{item.checklist_item_id}</span>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleUnbind(item.golden_set_id, item.checklist_item_id)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </li>
                ))}
            </ul>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
