import { apiClient } from '../apiClient';

// ── Request types ────────────────────────────────────────────────────────────

export type ChatRole = 'system' | 'user' | 'assistant';

export interface ChatMessage {
  role: ChatRole;
  content: string;
}

export interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  stream?: boolean;
  temperature?: number;
  max_tokens?: number;
}

export interface TextCompletionRequest {
  model: string;
  prompt: string;
  stream?: boolean;
  temperature?: number;
  max_tokens?: number;
}

// ── Response types (OpenAI-compatible) ───────────────────────────────────────

export interface ModelInfo {
  id: string;
  object: string;
  created: number;
  owned_by: string;
}

export interface ModelsListResponse {
  object: string;
  data: ModelInfo[];
}

export interface ChatCompletionChoice {
  index: number;
  message: ChatMessage;
  finish_reason: string | null;
}

export interface TextCompletionChoice {
  index: number;
  text: string;
  finish_reason: string | null;
}

export interface CompletionUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: ChatCompletionChoice[];
  usage: CompletionUsage;
}

export interface TextCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: TextCompletionChoice[];
  usage: CompletionUsage;
}

// ── API ───────────────────────────────────────────────────────────────────────

export const inferenceApi = {
  /** GET /openai/v1/models — list available models */
  listModels: async (): Promise<ModelsListResponse> => {
    return apiClient.get<ModelsListResponse>('/openai/v1/models');
  },

  /** POST /openai/v1/chat/completions — create a chat completion */
  createChatCompletion: async (
    body: ChatCompletionRequest
  ): Promise<ChatCompletionResponse> => {
    return apiClient.post<ChatCompletionResponse>('/openai/v1/chat/completions', body);
  },

  /** POST /openai/v1/completions — create a text completion */
  createCompletion: async (
    body: TextCompletionRequest
  ): Promise<TextCompletionResponse> => {
    return apiClient.post<TextCompletionResponse>('/openai/v1/completions', body);
  },
};
