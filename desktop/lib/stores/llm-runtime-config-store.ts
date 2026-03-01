import {
  type AvailableModel,
  type LlmRuntimeConfig,
  type ProviderId,
  getLlmRuntimeConfig,
  listAvailableModels,
  setLlmRuntimeConfig,
} from "@/lib/api/chat";

import { create } from "zustand";

interface LlmRuntimeConfigState {
  config: LlmRuntimeConfig;
  availableModels: AvailableModel[];
  selected: AvailableModel | null;
  loading: boolean;
  error: string | null;
  bootstrap: () => Promise<void>;
  saveConfig: (nextConfig: LlmRuntimeConfig) => Promise<LlmRuntimeConfig>;
  refreshAvailableModels: () => Promise<void>;
  setSelected: (provider: ProviderId, model: string) => void;
}

const EMPTY_PROVIDER = {
  apiKey: "",
  baseUrl: "",
  models: [],
  headers: [],
};

const EMPTY_CONFIG: LlmRuntimeConfig = {
  providers: {
    bigmodel: { ...EMPTY_PROVIDER },
    openai: { ...EMPTY_PROVIDER },
    anthropic: { ...EMPTY_PROVIDER },
  },
};

function findModel(
  available: AvailableModel[],
  provider: ProviderId,
  model: string
): AvailableModel | null {
  return (
    available.find((item) => item.provider === provider && item.model === model) ??
    null
  );
}

function chooseSelected(
  available: AvailableModel[],
  current: AvailableModel | null
): AvailableModel | null {
  if (available.length === 0) {
    return null;
  }
  if (current) {
    const matched = findModel(available, current.provider, current.model);
    if (matched) {
      return matched;
    }
  }
  return available[0];
}

export const useLlmRuntimeConfigStore = create<LlmRuntimeConfigState>((set, get) => ({
  config: EMPTY_CONFIG,
  availableModels: [],
  selected: null,
  loading: false,
  error: null,

  bootstrap: async () => {
    set({ loading: true, error: null });
    try {
      const [config, availableModels] = await Promise.all([
        getLlmRuntimeConfig(),
        listAvailableModels(),
      ]);
      set((state) => ({
        config,
        availableModels,
        selected: chooseSelected(availableModels, state.selected),
        loading: false,
      }));
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : "Failed to load LLM config",
      });
    }
  },

  saveConfig: async (nextConfig) => {
    set({ loading: true, error: null });
    try {
      const normalized = await setLlmRuntimeConfig(nextConfig);
      const availableModels = await listAvailableModels();
      set((state) => ({
        config: normalized,
        availableModels,
        selected: chooseSelected(availableModels, state.selected),
        loading: false,
      }));
      return normalized;
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to save LLM config";
      set({ loading: false, error: message });
      throw error;
    }
  },

  refreshAvailableModels: async () => {
    try {
      const availableModels = await listAvailableModels();
      set((state) => ({
        availableModels,
        selected: chooseSelected(availableModels, state.selected),
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to list available models",
      });
    }
  },

  setSelected: (provider, model) => {
    const { availableModels } = get();
    const next = findModel(availableModels, provider, model);
    set({ selected: next });
  },
}));
