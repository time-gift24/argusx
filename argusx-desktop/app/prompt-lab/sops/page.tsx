"use client";

import { useState, useEffect } from "react";
import { Plus, Pencil, Trash2, Eye, CheckCircle, XCircle } from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  listSops,
  createSop,
  updateSop,
  deleteSop,
  type Sop,
  type CreateSopInput,
} from "@/lib/api/prompt-lab";

export default function SopsPage() {
  const router = useRouter();
  const [sops, setSops] = useState<Sop[]>([]);
  const [loading, setLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [sopId, setSopId] = useState("");
  const [name, setName] = useState("");
  const [status, setStatus] = useState<"active" | "inactive" | "draft">("draft");
  const [submitting, setSubmitting] = useState(false);
  const [editingSop, setEditingSop] = useState<Sop | null>(null);

  const loadSops = () => {
    listSops({}).then((data) => {
      setSops(data);
      setLoading(false);
    });
  };

  useEffect(() => {
    loadSops();
  }, []);

  const handleCreate = async () => {
    if (!sopId.trim() || !name.trim()) return;
    setSubmitting(true);
    try {
      const input: CreateSopInput = {
        sop_id: sopId.trim(),
        name: name.trim(),
        status,
      };
      const newSop = await createSop(input);
      setSops((prev) => [newSop, ...prev]);
      setIsCreating(false);
      setSopId("");
      setName("");
      setStatus("draft");
    } finally {
      setSubmitting(false);
    }
  };

  const handleEdit = (sop: Sop) => {
    setEditingSop(sop);
    setSopId(sop.sop_id);
    setName(sop.name);
    setStatus(sop.status);
    setIsCreating(true);
  };

  const handleUpdate = async () => {
    if (!editingSop || !sopId.trim() || !name.trim()) return;
    setSubmitting(true);
    try {
      const updated = await updateSop({
        id: editingSop.id,
        sop_id: sopId.trim(),
        name: name.trim(),
        status,
      });
      setSops((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
      setIsCreating(false);
      setEditingSop(null);
      setSopId("");
      setName("");
      setStatus("draft");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: number) => {
    await deleteSop(id);
    setSops((prev) => prev.filter((s) => s.id !== id));
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">SOPs</h1>
        {!isCreating && (
          <Button variant="outline" onClick={() => setIsCreating(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Add SOP
          </Button>
        )}
      </div>

      <div className="grid gap-4">
        {/* Create/Edit Form Card */}
        {isCreating && (
          <Card>
            <CardHeader>
              <CardTitle>{editingSop ? "Edit SOP" : "New SOP"}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <Field>
                  <FieldLabel>SOP ID</FieldLabel>
                  <Input
                    value={sopId}
                    onChange={(e) => setSopId(e.target.value)}
                    placeholder="e.g., SOP-001"
                    autoFocus
                  />
                </Field>
                <Field>
                  <FieldLabel>Name</FieldLabel>
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g., Database Backup Procedure"
                  />
                </Field>
                <Field>
                  <FieldLabel>Status</FieldLabel>
                  <select
                    className="w-full border rounded-md px-3 py-2 bg-background text-foreground"
                    value={status}
                    onChange={(e) => setStatus(e.target.value as "active" | "inactive" | "draft")}
                  >
                    <option value="draft">Draft</option>
                    <option value="active">Active</option>
                    <option value="inactive">Inactive</option>
                  </select>
                </Field>
                <div className="flex justify-end gap-2">
                  <Button
                    variant="outline"
                    onClick={() => {
                      setIsCreating(false);
                      setEditingSop(null);
                      setSopId("");
                      setName("");
                      setStatus("draft");
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    onClick={editingSop ? handleUpdate : handleCreate}
                    disabled={submitting || !sopId.trim() || !name.trim()}
                  >
                    {submitting
                      ? editingSop
                        ? "Updating..."
                        : "Creating..."
                      : editingSop
                        ? "Update"
                        : "Create"}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Existing SOPs */}
        {sops.length === 0 && !isCreating && (
          <Card>
            <CardContent className="py-8 text-center text-muted-foreground">
              No SOPs found. Click "Add SOP" to create one.
            </CardContent>
          </Card>
        )}

        {sops.map((sop) => (
          <Card
            key={sop.id}
            className="cursor-pointer hover:bg-muted/50 transition-colors"
            onClick={() => router.push(`/prompt-lab/sops/${sop.id}`)}
          >
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                {sop.status === "active" ? (
                  <CheckCircle className="h-5 w-5 text-green-500" />
                ) : sop.status === "inactive" ? (
                  <XCircle className="h-5 w-5 text-muted-foreground" />
                ) : (
                  <XCircle className="h-5 w-5 text-yellow-500" />
                )}
                {sop.name}
              </CardTitle>
              <div className="flex items-center gap-2">
                <Badge
                  variant={
                    sop.status === "active"
                      ? "default"
                      : sop.status === "inactive"
                        ? "secondary"
                        : "outline"
                  }
                >
                  {sop.status}
                </Badge>
                <Badge variant="outline">v{sop.version}</Badge>
              </div>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground mb-3">
                <span className="font-mono">ID: {sop.sop_id}</span>
                {sop.ticket_id && <span className="ml-2">Ticket: {sop.ticket_id}</span>}
              </p>

              {/* Timestamps */}
              <div className="flex flex-wrap gap-4 text-xs text-muted-foreground border-t pt-3 mt-3">
                <div className="ml-auto">
                  Created: {new Date(sop.created_at).toLocaleDateString()}
                </div>
                <div>
                  Updated: {new Date(sop.updated_at).toLocaleDateString()}
                </div>
              </div>

              <div className="flex justify-end gap-2 mt-4">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    router.push(`/prompt-lab/sops/${sop.id}`);
                  }}
                  aria-label="View SOP"
                >
                  <Eye className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleEdit(sop);
                  }}
                  aria-label="Edit SOP"
                >
                  <Pencil className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  aria-label="Delete SOP"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(sop.id);
                  }}
                >
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
