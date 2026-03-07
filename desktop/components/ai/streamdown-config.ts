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

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
export const sharedStreamdownClassName = AI_STREAMDOWN_CLASSNAME;

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
export const sharedStreamdownShikiTheme = [
  "github-light",
  "github-dark-high-contrast",
] as const;

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
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

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
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

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
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

/**
 * @deprecated Preserved only as legacy Streamdown customization reference.
 * Runtime callsites now use default Streamdown rendering without this layer.
 */
export { sharedMathPlugin, sharedMermaidPlugin, sharedStreamdownPlugins };
