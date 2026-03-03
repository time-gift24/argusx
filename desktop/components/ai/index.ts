export { CHAT_STYLES, getRuntimeSurfaceClass } from "./styles";
export type { RuntimeSurfaceVariant } from "./styles";
export { shouldHighlightFence } from "./highlight-policy";
export { StreamdownCode } from "./streamdown-code";
export { STREAMDOWN_PLUGINS } from "./streamdown-plugins";

// Core UI components migrated from ai-elements
export { Message, MessageContent, MessageResponse } from "./message";
export type { MessageProps, MessageContentProps, MessageResponseProps } from "./message";
export { Reasoning, ReasoningContent, ReasoningHeader, ReasoningTitle, ReasoningTrigger } from "./reasoning";
export type { ReasoningProps, ReasoningContentProps, ReasoningHeaderProps, ReasoningTitleProps, ReasoningTriggerProps } from "./reasoning";
export { Plan, PlanContent, PlanDescription, PlanHeader, PlanTitle, PlanTrigger, PlanAction } from "./plan";
export type { PlanProps, PlanContentProps, PlanDescriptionProps, PlanHeaderProps, PlanTitleProps, PlanTriggerProps, PlanActionProps } from "./plan";
export { Tool, ToolContent, ToolHeader, ToolInput, ToolOutput } from "./tool";
export type { ToolProps, ToolContentProps, ToolHeaderProps, ToolInputProps, ToolOutputProps } from "./tool";
export { Terminal, TerminalContent, TerminalCopyButton } from "./terminal";
export type { TerminalProps, TerminalContentProps } from "./terminal";
export { CodeBlock, CodeBlockActions, CodeBlockCopyButton, CodeBlockDownloadButton, CodeBlockFilename, CodeBlockHeader, CodeBlockTitle } from "./code-block";
export type { CodeBlockProps, CodeBlockActionsProps, CodeBlockCopyButtonProps, CodeBlockDownloadButtonProps, CodeBlockFilenameProps, CodeBlockHeaderProps, CodeBlockTitleProps } from "./code-block";
