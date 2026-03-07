import { invoke } from "@tauri-apps/api/core";

export type ProviderKind = "openai_compatible" | "zai";

export interface ProviderProfileSummary {
  id: string;
  providerKind: ProviderKind;
  name: string;
  baseUrl: string;
  model: string;
  isDefault: boolean;
}

export interface SaveProviderProfileInput {
  id?: string;
  providerKind: ProviderKind;
  name: string;
  baseUrl: string;
  model: string;
  apiKey?: string;
  isDefault: boolean;
}

export interface TestProviderProfileInput {
  providerKind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey: string;
}

export interface ProviderConnectionResult {
  success: boolean;
  message: string;
}

export async function listProviderProfiles(): Promise<ProviderProfileSummary[]> {
  return invoke<ProviderProfileSummary[]>("list_provider_profiles");
}

export async function saveProviderProfile(
  input: SaveProviderProfileInput
): Promise<ProviderProfileSummary> {
  return invoke<ProviderProfileSummary>("save_provider_profile", { input });
}

export async function deleteProviderProfile(profileId: string): Promise<void> {
  await invoke("delete_provider_profile", { profileId });
}

export async function setDefaultProviderProfile(
  profileId: string
): Promise<ProviderProfileSummary> {
  return invoke<ProviderProfileSummary>("set_default_provider_profile", {
    profileId,
  });
}

export async function testProviderProfile(
  input: TestProviderProfileInput
): Promise<ProviderConnectionResult> {
  return invoke<ProviderConnectionResult>("test_provider_profile", { input });
}
