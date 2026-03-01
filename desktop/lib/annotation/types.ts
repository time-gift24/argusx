export type SourceType = "plain_field" | "rich_text_selection";

export type AnnotationLocation = {
  source_type: SourceType;
  panel: "basic_info" | "paragraph_detail";
  section_id: string;
  field_key: string;
  node_id: string;
  start_offset: number | null;
  end_offset: number | null;
  selected_text: string;
};

export type RuleFieldOption = {
  label: string;
  value: string;
};

export type RuleFieldSchema = {
  key: string;
  label: string;
  type: "text" | "textarea" | "select";
  required?: boolean;
  options?: RuleFieldOption[];
};

export type RuleCatalogItem = {
  code: string;
  label: string;
  description: string;
  version: number;
  schema: RuleFieldSchema[];
};
