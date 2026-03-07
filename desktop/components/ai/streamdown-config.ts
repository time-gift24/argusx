import type { ControlsConfig, IconMap, StreamdownTranslations } from "streamdown";

import {
  CheckIcon,
  CopyIcon,
  DownloadIcon,
  ExternalLinkIcon,
  Loader2Icon,
  Maximize2Icon,
  RotateCcwIcon,
  XIcon,
  ZoomInIcon,
  ZoomOutIcon,
} from "lucide-react";

import {
  sharedMathPlugin,
  sharedMermaidPlugin,
  sharedStreamdownPlugins,
} from "@/components/ai/streamdown-plugins";
import { AI_STREAMDOWN_CLASSNAME } from "@/components/ai/styles";

export const sharedStreamdownClassName = AI_STREAMDOWN_CLASSNAME;

export const sharedStreamdownControls = {
  code: {
    copy: true,
    download: true,
  },
  mermaid: {
    copy: true,
    download: true,
    fullscreen: true,
    panZoom: true,
  },
} satisfies ControlsConfig;

export const sharedStreamdownIcons = {
  CheckIcon,
  CopyIcon,
  DownloadIcon,
  ExternalLinkIcon,
  Loader2Icon,
  Maximize2Icon,
  RotateCcwIcon,
  XIcon,
  ZoomInIcon,
  ZoomOutIcon,
} satisfies Partial<IconMap>;

export const sharedStreamdownTranslations = {
  copyCode: "Copy code",
  downloadFile: "Download code",
  downloadDiagram: "Download diagram",
  downloadDiagramAsMmd: "Download diagram source",
  downloadDiagramAsPng: "Download diagram as PNG",
  downloadDiagramAsSvg: "Download diagram as SVG",
  exitFullscreen: "Exit fullscreen",
  mermaidFormatMmd: "Source",
  mermaidFormatPng: "PNG",
  mermaidFormatSvg: "SVG",
  viewFullscreen: "View fullscreen",
} satisfies Partial<StreamdownTranslations>;

export { sharedMathPlugin, sharedMermaidPlugin, sharedStreamdownPlugins };
