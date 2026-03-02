export const mockReviewData = {
  docId: "doc-001",
  basicInfo: {
    case_title: "某行政处罚案件",
    case_summary: "这是用于标注流程联调的示例摘要。",
  },
  sop: {
    sop_id: "sop-001",
    name: "行政处罚 SOP 样例",
    detect: [
      { sop_step_id: 11, name: "核对案件事实", version: 1 },
    ],
    handle: [
      { sop_step_id: 21, name: "出具处理意见", version: 1 },
    ],
    verification: [
      { sop_step_id: 31, name: "复核处罚依据", version: 1 },
    ],
    rollback: [
      { sop_step_id: 41, name: "回退审批流程", version: 1 },
    ],
    step_details: {
      11: {
        id: 11,
        name: "核对案件事实",
        operation: "<p>核对事实描述与证据材料。</p>",
        verification: "<p>确认事实与原文一致。</p>",
        impact_analysis: "<p>评估事实偏差对处罚结论影响。</p>",
        rollback: "<p>发现偏差时回退至事实补充环节。</p>",
      },
      21: {
        id: 21,
        name: "出具处理意见",
        operation: "<p>依据调查结果拟定处理意见。</p>",
        verification: "<p>核验意见与法条适配性。</p>",
        impact_analysis: "<p>评估处理意见执行影响。</p>",
        rollback: "<p>意见不一致时回退至调查环节。</p>",
      },
      31: {
        id: 31,
        name: "复核处罚依据",
        operation: "<p>对处罚依据进行复核。</p>",
        verification: "<p>核实法条引用完整性。</p>",
        impact_analysis: "<p>分析复核结果对处罚决定影响。</p>",
        rollback: "<p>复核失败时回退到处理意见环节。</p>",
      },
      41: {
        id: 41,
        name: "回退审批流程",
        operation: "<p>发起审批回退并记录原因。</p>",
        verification: "<p>确认回退节点正确。</p>",
        impact_analysis: "<p>评估回退对时效和流程影响。</p>",
        rollback: "<p>必要时撤销回退并恢复原流程。</p>",
      },
    },
  },
};
