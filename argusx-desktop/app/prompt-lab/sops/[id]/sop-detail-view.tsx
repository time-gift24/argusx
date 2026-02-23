"use client";

import { useState, useEffect } from "react";
import { ArrowLeft, Plus, Pencil, Trash2, CheckCircle, XCircle } from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  getSop,
  updateSop,
  listSopSteps,
  createSopStep,
  updateSopStep,
  deleteSopStep,
  type Sop,
  type SopStep,
  type UpdateSopInput,
  type CreateSopStepInput,
} from "@/lib/api/prompt-lab";

interface SopDetailViewProps {
  id: string;
}

export function SopDetailView({ id }: SopDetailViewProps) {
  const router = useRouter();
  const [sop, setSop] = useState<Sop | null>(null);
  const [steps, setSteps] = useState<SopStep[]>([]);
  const [loading, setLoading] = useState(true);
  const [isEditing, setIsEditing] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  // SOP edit form
  const [sopId, setSopId] = useState("");
  const [name, setName] = useState("");
  const [ticketId, setTicketId] = useState("");
  const [status, setStatus] = useState<"active" | "inactive" | "draft">("draft");

  // SOP Step form
  const [isCreatingStep, setIsCreatingStep] = useState(false);
  const [editingStep, setEditingStep] = useState<SopStep | null>(null);
  const [stepName, setStepName] = useState("");
  const [stepOperation, setStepOperation] = useState("");
  const [stepVerification, setStepVerification] = useState("");
  const [stepImpactAnalysis, setStepImpactAnalysis] = useState("");
  const [stepRollback, setStepRollback] = useState("");

  const sopIdNum = parseInt(id, 10);

  const loadData = () => {
    Promise.all([
      getSop(sopIdNum),
      listSopSteps({ sop_id: id }),
    ]).then(([sopData, stepsData]) => {
      setSop(sopData);
      setSteps(stepsData);
      // Initialize edit form
      setSopId(sopData.sop_id);
      setName(sopData.name);
      setTicketId(sopData.ticket_id || "");
      setStatus(sopData.status);
      setLoading(false);
    });
  };

  useEffect(() => {
    loadData();
  }, [id]);

  const handleUpdateSop = async () => {
    if (!sopId.trim() || !name.trim()) return;
    setSubmitting(true);
    try {
      const input: UpdateSopInput = {
        id: sopIdNum,
        sop_id: sopId.trim(),
        name: name.trim(),
        ticket_id: ticketId.trim() || undefined,
        status,
      };
      const updated = await updateSop(input);
      setSop(updated);
      setIsEditing(false);
    } finally {
      setSubmitting(false);
    }
  };

  const handleCreateStep = async () => {
    if (!stepName.trim()) return;
    setSubmitting(true);
    try {
      const input: CreateSopStepInput = {
        sop_id: sop?.sop_id || "",
        name: stepName.trim(),
        operation: stepOperation.trim() ? JSON.parse(stepOperation.trim()) : null,
        verification: stepVerification.trim() ? JSON.parse(stepVerification.trim()) : null,
        impact_analysis: stepImpactAnalysis.trim() ? JSON.parse(stepImpactAnalysis.trim()) : null,
        rollback: stepRollback.trim() ? JSON.parse(stepRollback.trim()) : null,
      };
      const newStep = await createSopStep(input);
      setSteps((prev) => [...prev, newStep]);
      setIsCreatingStep(false);
      resetStepForm();
    } catch (e) {
      alert("Invalid JSON in step fields");
    } finally {
      setSubmitting(false);
    }
  };

  const handleEditStep = (step: SopStep) => {
    setEditingStep(step);
    setStepName(step.name);
    setStepOperation(step.operation ? JSON.stringify(step.operation, null, 2) : "");
    setStepVerification(step.verification ? JSON.stringify(step.verification, null, 2) : "");
    setStepImpactAnalysis(step.impact_analysis ? JSON.stringify(step.impact_analysis, null, 2) : "");
    setStepRollback(step.rollback ? JSON.stringify(step.rollback, null, 2) : "");
    setIsCreatingStep(true);
  };

  const handleUpdateStep = async () => {
    if (!editingStep || !stepName.trim()) return;
    setSubmitting(true);
    try {
      const input = {
        id: editingStep.id,
        name: stepName.trim(),
        operation: stepOperation.trim() ? JSON.parse(stepOperation.trim()) : null,
        verification: stepVerification.trim() ? JSON.parse(stepVerification.trim()) : null,
        impact_analysis: stepImpactAnalysis.trim() ? JSON.parse(stepImpactAnalysis.trim()) : null,
        rollback: stepRollback.trim() ? JSON.parse(stepRollback.trim()) : null,
      };
      const updated = await updateSopStep(input);
      setSteps((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
      setIsCreatingStep(false);
      setEditingStep(null);
      resetStepForm();
    } catch (e) {
      alert("Invalid JSON in step fields");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDeleteStep = async (stepId: number) => {
    await deleteSopStep(stepId);
    setSteps((prev) => prev.filter((s) => s.id !== stepId));
  };

  const resetStepForm = () => {
    setStepName("");
    setStepOperation("");
    setStepVerification("");
    setStepImpactAnalysis("");
    setStepRollback("");
  };

  if (loading) {
    return <div>Loading...</div>;
  }

  if (!sop) {
    return <div>SOP not found</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" onClick={() => router.push("/prompt-lab/sops")}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back
        </Button>
        <h1 className="text-2xl font-bold">SOP Details</h1>
      </div>

      {/* SOP Info Card */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-3">
            <CardTitle>SOP Information</CardTitle>
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
          {!isEditing && (
            <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
              <Pencil className="h-4 w-4 mr-2" />
              Edit
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {isEditing ? (
            <div className="space-y-4">
              <Field>
                <FieldLabel>SOP ID</FieldLabel>
                <Input value={sopId} onChange={(e) => setSopId(e.target.value)} />
              </Field>
              <Field>
                <FieldLabel>Name</FieldLabel>
                <Input value={name} onChange={(e) => setName(e.target.value)} />
              </Field>
              <Field>
                <FieldLabel>Ticket ID</FieldLabel>
                <Input value={ticketId} onChange={(e) => setTicketId(e.target.value)} />
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
                <Button variant="outline" onClick={() => setIsEditing(false)}>
                  Cancel
                </Button>
                <Button onClick={handleUpdateSop} disabled={submitting}>
                  {submitting ? "Saving..." : "Save"}
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <span className="text-sm text-muted-foreground">SOP ID:</span>
                  <span className="ml-2 font-mono">{sop.sop_id}</span>
                </div>
                <div>
                  <span className="text-sm text-muted-foreground">Name:</span>
                  <span className="ml-2 font-semibold">{sop.name}</span>
                </div>
                <div>
                  <span className="text-sm text-muted-foreground">Ticket ID:</span>
                  <span className="ml-2">{sop.ticket_id || "-"}</span>
                </div>
                <div>
                  <span className="text-sm text-muted-foreground">Version:</span>
                  <span className="ml-2">{sop.version}</span>
                </div>
                <div>
                  <span className="text-sm text-muted-foreground">Created:</span>
                  <span className="ml-2">{new Date(sop.created_at).toLocaleString()}</span>
                </div>
                <div>
                  <span className="text-sm text-muted-foreground">Updated:</span>
                  <span className="ml-2">{new Date(sop.updated_at).toLocaleString()}</span>
                </div>
              </div>

              {/* SOP JSON Fields */}
              <div className="grid grid-cols-2 gap-4 mt-4 pt-4 border-t">
                {sop.detect && (
                  <div>
                    <div className="text-sm text-muted-foreground mb-1">Detect</div>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(sop.detect, null, 2)}
                    </pre>
                  </div>
                )}
                {sop.handle && (
                  <div>
                    <div className="text-sm text-muted-foreground mb-1">Handle</div>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(sop.handle, null, 2)}
                    </pre>
                  </div>
                )}
                {sop.verification && (
                  <div>
                    <div className="text-sm text-muted-foreground mb-1">Verification</div>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(sop.verification, null, 2)}
                    </pre>
                  </div>
                )}
                {sop.rollback && (
                  <div>
                    <div className="text-sm text-muted-foreground mb-1">Rollback</div>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(sop.rollback, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* SOP Steps Card */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>SOP Steps</CardTitle>
          {!isCreatingStep && (
            <Button variant="outline" size="sm" onClick={() => setIsCreatingStep(true)}>
              <Plus className="h-4 w-4 mr-2" />
              Add Step
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {/* Create/Edit Step Form */}
          {isCreatingStep && (
            <div className="space-y-4 mb-6 p-4 border rounded-lg bg-muted/30">
              <h3 className="font-semibold">
                {editingStep ? "Edit Step" : "New Step"}
              </h3>
              <Field>
                <FieldLabel>Step Name</FieldLabel>
                <Input
                  value={stepName}
                  onChange={(e) => setStepName(e.target.value)}
                  placeholder="e.g., Backup Database"
                  autoFocus
                />
              </Field>
              <Field>
                <FieldLabel>Operation (JSON)</FieldLabel>
                <Textarea
                  value={stepOperation}
                  onChange={(e) => setStepOperation(e.target.value)}
                  placeholder='{"command": "pg_dump", "args": ["..."]}'
                  rows={3}
                />
              </Field>
              <Field>
                <FieldLabel>Verification (JSON)</FieldLabel>
                <Textarea
                  value={stepVerification}
                  onChange={(e) => setStepVerification(e.target.value)}
                  placeholder='{"check": "connection", "expected": "active"}'
                  rows={3}
                />
              </Field>
              <Field>
                <FieldLabel>Impact Analysis (JSON)</FieldLabel>
                <Textarea
                  value={stepImpactAnalysis}
                  onChange={(e) => setStepImpactAnalysis(e.target.value)}
                  placeholder='{"risk": "low", "downtime": "0"}'
                  rows={3}
                />
              </Field>
              <Field>
                <FieldLabel>Rollback (JSON)</FieldLabel>
                <Textarea
                  value={stepRollback}
                  onChange={(e) => setStepRollback(e.target.value)}
                  placeholder='{"command": "pg_restore", "args": ["..."]}'
                  rows={3}
                />
              </Field>
              <div className="flex justify-end gap-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setIsCreatingStep(false);
                    setEditingStep(null);
                    resetStepForm();
                  }}
                >
                  Cancel
                </Button>
                <Button
                  onClick={editingStep ? handleUpdateStep : handleCreateStep}
                  disabled={submitting || !stepName.trim()}
                >
                  {submitting
                    ? editingStep
                      ? "Updating..."
                      : "Creating..."
                    : editingStep
                      ? "Update"
                      : "Create"}
                </Button>
              </div>
            </div>
          )}

          {/* Steps List */}
          {steps.length === 0 && !isCreatingStep ? (
            <p className="text-center text-muted-foreground py-4">
              No steps defined. Click "Add Step" to create one.
            </p>
          ) : (
            <div className="space-y-4">
              {steps.map((step, index) => (
                <Card key={step.id} className="bg-muted/30">
                  <CardHeader className="flex flex-row items-center justify-between py-3">
                    <div className="flex items-center gap-3">
                      <Badge variant="outline">{index + 1}</Badge>
                      <span className="font-semibold">{step.name}</span>
                      <Badge variant="outline">v{step.version}</Badge>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleEditStep(step)}
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteStep(step.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </CardHeader>
                  <CardContent className="py-2">
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      {step.operation && (
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">Operation</div>
                          <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                            {JSON.stringify(step.operation, null, 2)}
                          </pre>
                        </div>
                      )}
                      {step.verification && (
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">Verification</div>
                          <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                            {JSON.stringify(step.verification, null, 2)}
                          </pre>
                        </div>
                      )}
                      {step.impact_analysis && (
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">Impact Analysis</div>
                          <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                            {JSON.stringify(step.impact_analysis, null, 2)}
                          </pre>
                        </div>
                      )}
                      {step.rollback && (
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">Rollback</div>
                          <pre className="text-xs bg-background p-2 rounded overflow-x-auto">
                            {JSON.stringify(step.rollback, null, 2)}
                          </pre>
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
