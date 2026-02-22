"use client";

import { useState, useEffect } from "react";
import { Plus, Pencil, Trash2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  listChecklistItems,
  createChecklistItem,
  deleteChecklistItem,
  type ChecklistItem,
  type CreateChecklistItemInput,
} from "@/lib/api/prompt-lab";

export default function ChecklistPage() {
  const [items, setItems] = useState<ChecklistItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [targetLevel, setTargetLevel] = useState<"step" | "sop">("step");
  const [submitting, setSubmitting] = useState(false);

  const loadItems = () => {
    listChecklistItems({}).then((data) => {
      setItems(data);
      setLoading(false);
    });
  };

  useEffect(() => {
    loadItems();
  }, []);

  const handleCreate = async () => {
    if (!name.trim() || !prompt.trim()) return;
    setSubmitting(true);
    try {
      const input: CreateChecklistItemInput = {
        name: name.trim(),
        prompt: prompt.trim(),
        target_level: targetLevel,
        status: "active",
      };
      const newItem = await createChecklistItem(input);
      setItems([newItem, ...items]);
      setIsCreating(false);
      setName("");
      setPrompt("");
      setTargetLevel("step");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: number) => {
    await deleteChecklistItem(id);
    setItems(items.filter((i) => i.id !== id));
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Checklist Items</h1>
        {!isCreating && (
          <Button variant="outline" onClick={() => setIsCreating(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Add Item
          </Button>
        )}
      </div>

      <div className="grid gap-4">
        {/* Create Form Card */}
        {isCreating && (
          <Card>
            <CardHeader>
              <CardTitle>New Checklist Item</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <Field>
                  <FieldLabel>Name</FieldLabel>
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g., Check JSON syntax"
                    autoFocus
                  />
                </Field>
                <Field>
                  <FieldLabel>Prompt</FieldLabel>
                  <Textarea
                    value={prompt}
                    onChange={(e) => setPrompt(e.target.value)}
                    placeholder="e.g., Validate that the output is valid JSON"
                    rows={3}
                  />
                </Field>
                <Field>
                  <FieldLabel>Target Level</FieldLabel>
                  <select
                    className="w-full border rounded-md px-3 py-2 bg-background"
                    value={targetLevel}
                    onChange={(e) => setTargetLevel(e.target.value as "step" | "sop")}
                  >
                    <option value="step">Step</option>
                    <option value="sop">SOP</option>
                  </select>
                </Field>
                <div className="flex justify-end gap-2">
                  <Button variant="outline" onClick={() => setIsCreating(false)}>
                    Cancel
                  </Button>
                  <Button
                    onClick={handleCreate}
                    disabled={submitting || !name.trim() || !prompt.trim()}
                  >
                    {submitting ? "Creating..." : "Create"}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Existing Items */}
        {items.map((item) => (
          <Card key={item.id}>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>{item.name}</CardTitle>
              <div className="flex items-center gap-2">
                <Badge variant={item.status === "active" ? "default" : "secondary"}>
                  {item.status}
                </Badge>
                <Badge variant="outline">{item.target_level}</Badge>
              </div>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground line-clamp-2">{item.prompt}</p>
              <div className="flex justify-end gap-2 mt-4">
                <Button variant="outline" size="sm">
                  <Pencil className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => handleDelete(item.id)}>
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
