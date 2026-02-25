"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { ChevronRight, ChevronDown, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

// Types matching Rust types
interface SopStepRef {
  sop_step_id: number;
  name: string;
}

interface SopStage {
  name: string;
  steps: SopStepRef[];
}

interface Sop {
  id: number;
  sop_id: string;
  name: string;
}

interface SopData {
  sop: Sop;
  detect_stages: SopStage[];
  handle_stages: SopStage[];
  verification_stages: SopStage[];
  rollback_stages: SopStage[];
}

interface ChecklistItem {
  id: number;
  name: string;
  prompt: string;
  context_type: string;
}

interface SopStep {
  id: number;
  sop_id: string;
  name: string;
  version: number;
  operation: string | null;
  verification: string | null;
  impact_analysis: string | null;
  rollback: string | null;
  created_at: string;
  updated_at: string;
}

// Phase configuration
const PHASES = [
  { key: "detect_stages", label: "Detect", color: "text-blue-500" },
  { key: "handle_stages", label: "Handle", color: "text-orange-500" },
  { key: "verification_stages", label: "Verification", color: "text-green-500" },
  { key: "rollback_stages", label: "Rollback", color: "text-red-500" },
] as const;

interface SopClientProps {
  sopId: string;
}

export default function SopClient({ sopId }: SopClientProps) {

  const [sopData, setSopData] = useState<SopData | null>(null);
  const [selectedStep, setSelectedStep] = useState<number | null>(null);
  const [selectedStepData, setSelectedStepData] = useState<SopStep | null>(null);
  const [checklistItems, setChecklistItems] = useState<ChecklistItem[]>([]);
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [isLoadingStepDetails, setIsLoadingStepDetails] = useState(false);
  const [isLoadingChecklist, setIsLoadingChecklist] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedPhases, setExpandedPhases] = useState<Record<string, boolean>>({});

  // Load SOP data on mount
  useEffect(() => {
    if (!sopId) return;

    const loadSopData = async () => {
      try {
        setError(null);
        const data = await invoke<SopData>("get_sop_with_steps", { sopId });
        setSopData(data);

        // Expand all phases by default
        const expanded: Record<string, boolean> = {};
        PHASES.forEach((phase) => {
          expanded[phase.key] = true;
        });
        setExpandedPhases(expanded);
      } catch (error) {
        console.error("Failed to load SOP data:", error);
        setError(error instanceof Error ? error.message : String(error));
      } finally {
        setLoading(false);
      }
    };

    loadSopData();
  }, [sopId]);

  // Load checklist items when step is selected
  useEffect(() => {
    if (!selectedStep) {
      setChecklistItems([]);
      return;
    }

    const loadChecklistItems = async () => {
      setIsLoadingChecklist(true);
      try {
        const items = await invoke<ChecklistItem[]>("get_checklist_items_by_step", {
          stepId: selectedStep,
        });
        setChecklistItems(items);
      } catch (error) {
        console.error("Failed to load checklist items:", error);
        setChecklistItems([]);
      } finally {
        setIsLoadingChecklist(false);
      }
    };

    loadChecklistItems();
  }, [selectedStep]);

  // Load step details when step is selected
  useEffect(() => {
    if (!selectedStep) {
      setSelectedStepData(null);
      return;
    }

    const loadStepDetails = async () => {
      setIsLoadingStepDetails(true);
      try {
        const step = await invoke<SopStep>("get_sop_step_details", {
          stepId: selectedStep,
        });
        setSelectedStepData(step);
        setEditContent(step.operation || "");
      } catch (error) {
        console.error("Failed to load step details:", error);
        setSelectedStepData(null);
      } finally {
        setIsLoadingStepDetails(false);
      }
    };

    loadStepDetails();
  }, [selectedStep]);

  const togglePhase = (phaseKey: string) => {
    setExpandedPhases((prev) => ({
      ...prev,
      [phaseKey]: !prev[phaseKey],
    }));
  };

  const handleEdit = () => {
    setIsEditing(true);
    setEditContent(selectedStepData?.operation || "");
  };

  const handleSave = async () => {
    if (!selectedStep) return;

    try {
      const updated = await invoke<SopStep>("update_sop_step", {
        id: selectedStep,
        operation: editContent,
      });
      setSelectedStepData(updated);
      setIsEditing(false);
    } catch (error) {
      console.error("Failed to save step:", error);
    }
  };

  const handleCancel = () => {
    setIsEditing(false);
    setEditContent(selectedStepData?.operation || "");
  };

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <p className="text-red-500 font-medium">Error loading SOP</p>
          <p className="text-muted-foreground text-sm mt-1">{error}</p>
        </div>
      </div>
    );
  }

  if (!sopData) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-muted-foreground">SOP not found</p>
      </div>
    );
  }

  return (
    <div className="flex h-full gap-4 p-4">
      {/* Left Pane: Tree Navigation */}
      <Card className="w-64 flex-shrink-0 overflow-auto">
        <CardHeader className="pb-2">
          <CardTitle className="text-base">{sopData.sop.name}</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="space-y-1">
            {PHASES.map((phase) => {
              const stages = sopData[phase.key as keyof SopData] as SopStage[];
              const isExpanded = expandedPhases[phase.key];

              return (
                <Collapsible
                  key={phase.key}
                  open={isExpanded}
                  onOpenChange={() => togglePhase(phase.key)}
                >
                  <CollapsibleTrigger className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-sm font-medium hover:bg-accent">
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                    <span className={phase.color}>{phase.label}</span>
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <div className="ml-4 space-y-1">
                      {stages.map((stage) =>
                        stage.steps.map((step) => (
                          <button
                            key={step.sop_step_id}
                            onClick={() => setSelectedStep(step.sop_step_id)}
                            className={cn(
                              "flex w-full items-center gap-2 rounded px-2 py-1 text-sm text-left hover:bg-accent",
                              selectedStep === step.sop_step_id &&
                                "bg-accent"
                            )}
                          >
                            <span className="truncate">{step.name}</span>
                          </button>
                        ))
                      )}
                    </div>
                  </CollapsibleContent>
                </Collapsible>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Middle Pane: Step Content */}
      <Card className="flex-1 overflow-auto">
        <CardHeader className="pb-2">
          <CardTitle className="text-lg">
            {selectedStepData?.name || "Select a step"}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoadingStepDetails ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : selectedStepData ? (
            <div className="space-y-4">
              <div>
                <label className="text-sm font-medium">Operation</label>
                {isEditing ? (
                  <Textarea
                    value={editContent}
                    onChange={(e) => setEditContent(e.target.value)}
                    className="mt-2 min-h-[200px] font-mono text-sm"
                    placeholder="Enter operation content..."
                  />
                ) : (
                  <div className="mt-2 rounded border bg-muted/50 p-3 min-h-[100px]">
                    <pre className="whitespace-pre-wrap text-sm">
                      {selectedStepData.operation || "No operation defined"}
                    </pre>
                  </div>
                )}
              </div>

              <div className="flex gap-2">
                {isEditing ? (
                  <>
                    <Button onClick={handleSave} size="sm">
                      Save
                    </Button>
                    <Button
                      onClick={handleCancel}
                      variant="outline"
                      size="sm"
                    >
                      Cancel
                    </Button>
                  </>
                ) : (
                  <Button onClick={handleEdit} size="sm">
                    Edit
                  </Button>
                )}
              </div>

              {/* Additional fields */}
              <div className="space-y-4 pt-4 border-t">
                <div>
                  <label className="text-sm font-medium text-muted-foreground">
                    Verification
                  </label>
                  <pre className="mt-1 whitespace-pre-wrap text-sm text-muted-foreground">
                    {selectedStepData.verification || "No verification defined"}
                  </pre>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">
                    Impact Analysis
                  </label>
                  <pre className="mt-1 whitespace-pre-wrap text-sm text-muted-foreground">
                    {selectedStepData.impact_analysis || "No impact analysis defined"}
                  </pre>
                </div>
                <div>
                  <label className="text-sm font-medium text-muted-foreground">
                    Rollback
                  </label>
                  <pre className="mt-1 whitespace-pre-wrap text-sm text-muted-foreground">
                    {selectedStepData.rollback || "No rollback defined"}
                  </pre>
                </div>
              </div>
            </div>
          ) : (
            <p className="text-muted-foreground">
              Select a step from the left panel to view its details
            </p>
          )}
        </CardContent>
      </Card>

      {/* Right Pane: Checklist */}
      <Card className="w-72 flex-shrink-0 overflow-auto">
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Checklist</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          {isLoadingChecklist ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : checklistItems.length > 0 ? (
            <div className="space-y-2">
              {checklistItems.map((item) => (
                <div
                  key={item.id}
                  className="rounded p-2 hover:bg-accent/50"
                >
                  <p className="text-sm font-medium">{item.name}</p>
                  <p className="text-xs text-muted-foreground truncate">
                    {item.prompt}
                  </p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {selectedStep
                ? "No checklist items for this step"
                : "Select a step to view checklist"}
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
