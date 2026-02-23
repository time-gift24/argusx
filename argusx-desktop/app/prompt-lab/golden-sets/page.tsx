"use client";

import { useState, useEffect } from "react";
import { Plus, GripVertical, Trash2, CheckCircle, ArrowRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import {
  listGoldenSetItems,
  listChecklistItems,
  unbindGoldenSetItem,
  type GoldenSetItem,
  type ChecklistItem,
} from "@/lib/api/prompt-lab";

const goldenSets = [
  { id: 1, name: "Default Set" },
];

export default function GoldenSetsPage() {
  const router = useRouter();
  const [items, setItems] = useState<GoldenSetItem[]>([]);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      listGoldenSetItems(1),
      listChecklistItems({}),
    ]).then(([data, itemsData]) => {
      setItems(data);
      setChecklistItems(itemsData);
      setLoading(false);
    });
  }, []);

  const handleUnbind = async (goldenSetId: number, checklistItemId: number) => {
    await unbindGoldenSetItem(goldenSetId, checklistItemId);
    setItems(items.filter(
      (i) => !(i.golden_set_id === goldenSetId && i.checklist_item_id === checklistItemId)
    ));
  };

  const getChecklistName = (id: number) => {
    const item = checklistItems.find((i) => i.id === id);
    return item?.name || `Checklist Item #${id}`;
  };

  if (loading) {
    return <div className="p-8 text-center text-muted-foreground">Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Golden Sets</h1>
        <Button>
          <Plus className="h-4 w-4 mr-2" />
          Create Set
        </Button>
      </div>

      {goldenSets.map((set) => {
        const setItems = items.filter((i) => i.golden_set_id === set.id);

        return (
          <Card key={set.id} className="hover:shadow-md transition-shadow">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                {set.name}
                <Badge>{setItems.length} items</Badge>
              </CardTitle>
            </CardHeader>
            <CardContent>
              {setItems.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <p>No items in this golden set</p>
                  <Button
                    variant="link"
                    className="mt-2"
                    onClick={() => router.push("/prompt-lab/checklist")}
                  >
                    Add from Checklist <ArrowRight className="h-4 w-4 ml-1" />
                  </Button>
                </div>
              ) : (
                <ul className="space-y-2">
                  {setItems.map((item) => (
                    <li
                      key={item.checklist_item_id}
                      className="flex items-center justify-between p-3 rounded-md bg-muted hover:bg-muted/80 transition-colors"
                    >
                      <div className="flex items-center gap-3">
                        <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />
                        <CheckCircle className="h-4 w-4 text-green-500" />
                        <div className="flex flex-col">
                          <span className="font-medium">{getChecklistName(item.checklist_item_id)}</span>
                          <span className="text-xs text-muted-foreground">ID: {item.checklist_item_id}</span>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => router.push(`/prompt-lab/checklist/${item.checklist_item_id}`)}
                        >
                          <ArrowRight className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleUnbind(item.golden_set_id, item.checklist_item_id)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
