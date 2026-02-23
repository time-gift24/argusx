"use client";

import { useState, useEffect } from "react";
import { Plus, Pencil, Trash2, Eye, CheckCircle, XCircle, Folder } from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  listChecklistItems,
  createChecklistItem,
  updateChecklistItem,
  deleteChecklistItem,
  listCheckResults,
  type ChecklistItem,
  type CreateChecklistItemInput,
  type CheckResult,
} from "@/lib/api/prompt-lab";

export default function ChecklistPage() {
  const router = useRouter();
  const [items, setItems] = useState<ChecklistItem[]>([]);
  const [results, setResults] = useState<CheckResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [targetLevel, setTargetLevel] = useState<"step" | "sop">("step");
  const [submitting, setSubmitting] = useState(false);
  const [editingItem, setEditingItem] = useState<ChecklistItem | null>(null);

  const loadItems = () => {
    Promise.all([
      listChecklistItems({}),
      listCheckResults({}),
    ]).then(([data, resultsData]) => {
      setItems(data);
      setResults(resultsData);
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
      setItems((prev) => [newItem, ...prev]);
      setIsCreating(false);
      setName("");
      setPrompt("");
      setTargetLevel("step");
    } finally {
      setSubmitting(false);
    }
  };

  const handleEdit = (item: ChecklistItem) => {
    setEditingItem(item);
    setName(item.name);
    setPrompt(item.prompt);
    setTargetLevel(item.target_level);
    setIsCreating(true);
  };

  const handleUpdate = async () => {
    if (!editingItem || !name.trim() || !prompt.trim()) return;
    setSubmitting(true);
    try {
      const updated = await updateChecklistItem({
        id: editingItem.id,
        name: name.trim(),
        prompt: prompt.trim(),
        target_level: targetLevel,
      });
      setItems((prev) => prev.map((i) => (i.id === updated.id ? updated : i)));
      setIsCreating(false);
      setEditingItem(null);
      setName("");
      setPrompt("");
      setTargetLevel("step");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: number) => {
    await deleteChecklistItem(id);
    setItems((prev) => prev.filter((i) => i.id !== id));
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
        {/* Create/Edit Form Card */}
        {isCreating && (
          <Card>
            <CardHeader>
              <CardTitle>{editingItem ? "Edit Checklist Item" : "New Checklist Item"}</CardTitle>
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
                    className="w-full border rounded-md px-3 py-2 bg-background text-foreground"
                    value={targetLevel}
                    onChange={(e) => setTargetLevel(e.target.value as "step" | "sop")}
                  >
                    <option value="step">Step</option>
                    <option value="sop">SOP</option>
                  </select>
                </Field>
                <div className="flex justify-end gap-2">
                  <Button
                    variant="outline"
                    onClick={() => {
                      setIsCreating(false);
                      setEditingItem(null);
                      setName("");
                      setPrompt("");
                      setTargetLevel("step");
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    onClick={editingItem ? handleUpdate : handleCreate}
                    disabled={submitting || !name.trim() || !prompt.trim()}
                  >
                    {submitting
                      ? editingItem
                        ? "Updating..."
                        : "Creating..."
                      : editingItem
                        ? "Update"
                        : "Create"}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Existing Items */}
        {items.map((item) => {
          const itemResults = results.filter((r) => r.check_item_id === item.id);
          const passed = itemResults.filter((r) => r.is_pass).length;
          const failed = itemResults.filter((r) => !r.is_pass).length;

          return (
            <Card
              key={item.id}
              className="cursor-pointer hover:bg-muted/50 transition-colors"
              onClick={() => router.push(`/prompt-lab/checklist/${item.id}`)}
            >
              <CardHeader className="flex flex-row items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  {item.status === "active" ? (
                    <CheckCircle className="h-5 w-5 text-green-500" />
                  ) : (
                    <XCircle className="h-5 w-5 text-muted-foreground" />
                  )}
                  {item.name}
                </CardTitle>
                <div className="flex items-center gap-2">
                  <Badge variant={item.status === "active" ? "default" : "secondary"}>
                    {item.status}
                  </Badge>
                  <Badge variant="outline">{item.target_level}</Badge>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground line-clamp-2 mb-3">
                  {item.prompt.slice(0, 100)}
                  {item.prompt.length > 100 ? "..." : ""}
                </p>

                {/* 关联信息和时间 */}
                <div className="flex flex-wrap gap-4 text-xs text-muted-foreground border-t pt-3 mt-3">
                  <div className="flex items-center gap-1">
                    <Folder className="h-3 w-3" />
                    <span>Golden Set #1</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <CheckCircle className="h-3 w-3 text-green-500" />
                    <span>{passed} passed</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <XCircle className="h-3 w-3 text-red-500" />
                    <span>{failed} failed</span>
                  </div>
                  <div className="ml-auto">
                    {new Date(item.created_at).toLocaleDateString()}
                  </div>
                </div>

                <div className="flex justify-end gap-2 mt-4">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      router.push(`/prompt-lab/checklist/${item.id}`);
                    }}
                    aria-label="View item"
                  >
                    <Eye className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleEdit(item);
                    }}
                    aria-label="Edit item"
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    aria-label="Delete item"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(item.id);
                    }}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
