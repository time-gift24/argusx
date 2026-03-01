import { QuillReviewField } from "./quill-review-field";

const PARAGRAPH_FIELDS = [
  {
    fieldKey: "paragraph.summary",
    label: "段落摘要",
    text: "本段描述了事实背景与时间线。",
  },
  {
    fieldKey: "paragraph.basis",
    label: "法条依据",
    text: "援引了行政处罚法第 xx 条作为处理依据。",
  },
  {
    fieldKey: "paragraph.process",
    label: "程序描述",
    text: "记录了调查、告知和陈述申辩过程。",
  },
  {
    fieldKey: "paragraph.decision",
    label: "处理结论",
    text: "最终作出罚款并责令整改的决定。",
  },
];

export function ParagraphPanel() {
  return (
    <div className="space-y-3">
      {PARAGRAPH_FIELDS.map((field, index) => (
        <QuillReviewField
          key={field.fieldKey}
          sectionId={`paragraph-${index + 1}`}
          fieldKey={field.fieldKey}
          label={field.label}
          text={field.text}
        />
      ))}
    </div>
  );
}
