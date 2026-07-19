import type {
  AdminAssistantCandidate,
  AdminAssistantNote,
  AdminAssistantProfile,
  AdminKnowledgeCandidate,
  AdminKnowledgeCandidateDetails,
  AdminKnowledgeSource,
  Article,
  Category,
  ChatResponse,
  Guide,
} from "../types";

const API_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8000";
const DATA_LOAD_ERROR = "Не удалось загрузить данные. Проверьте подключение к API.";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${API_URL}${path}`, {
      headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
      ...init,
    });
  } catch {
    throw new Error(DATA_LOAD_ERROR);
  }

  if (!response.ok) {
    if ((init?.method ?? "GET").toUpperCase() === "GET") {
      throw new Error(DATA_LOAD_ERROR);
    }

    const body = await response.json().catch(() => ({ error: DATA_LOAD_ERROR }));
    throw new Error(body.error ?? DATA_LOAD_ERROR);
  }

  return response.json() as Promise<T>;
}

export const api = {
  categories: () => request<Category[]>("/api/categories"),
  articles: () => request<Article[]>("/api/articles"),
  article: (slug: string) => request<Article>(`/api/articles/${slug}`),
  guides: () => request<Guide[]>("/api/guides"),
  guide: (slug: string) => request<Guide>(`/api/guides/${slug}`),
  search: (q: string) =>
    request<Array<{ kind: string; title_ru: string; slug: string; summary_ru: string }>>(
      `/api/search?q=${encodeURIComponent(q)}`,
    ),
  chat: (message: string, session_id: string, page_context?: string) =>
    request<ChatResponse>("/api/chat", {
      method: "POST",
      body: JSON.stringify({ message, session_id, page_context }),
    }),
  unlock: (session_id: string, access_key: string) =>
    request<{ unlocked: boolean }>("/api/access/unlock", {
      method: "POST",
      body: JSON.stringify({ session_id, access_key }),
    }),
  adminLogin: (username: string, password: string) =>
    request<{ token: string }>("/api/admin/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  adminStats: (token: string) =>
    request<{
      content: { categories: number; articles: number; guides: number; sessions: number };
      rag_vectors: number;
    }>("/api/admin/stats", {
      headers: { authorization: `Bearer ${token}` },
    }),
  ragStatus: () =>
    request<{
      collection: string;
      vectors: number;
      embedding_provider: string;
      embedding_model: string;
      embedding_dimensions: number;
      embedding_fallback: string;
      qdrant_available: boolean;
      error: string | null;
    }>("/api/rag/status"),
  reindex: () => request<{ files: number; chunks: number }>("/api/rag/reindex", { method: "POST" }),
  adminKnowledgeSources: (token: string) =>
    request<AdminKnowledgeSource[]>("/api/admin/knowledge/sources", { headers: auth(token) }),
  adminCreateKnowledgeSource: (
    token: string,
    body: {
      title: string;
      source_type: "pasted_text" | "manual_note" | "url" | "file";
      raw_text?: string;
      original_path?: string;
      source_url?: string;
      trust_level: "official" | "semi_official" | "user_provided" | "unknown";
    },
  ) =>
    request<AdminKnowledgeSource>("/api/admin/knowledge/sources", {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify(body),
    }),
  adminScanDocs: (token: string) =>
    request<{ sources: number }>("/api/admin/knowledge/sources/scan-docs", {
      method: "POST",
      headers: auth(token),
    }),
  adminAnalyzeSource: (token: string, id: string) =>
    request<{ candidate: AdminKnowledgeCandidate }>(`/api/admin/knowledge/sources/${id}/analyze-now`, {
      method: "POST",
      headers: auth(token),
    }),
  adminIndexSource: (token: string, id: string) =>
    request<{ queued: boolean }>(`/api/admin/knowledge/sources/${id}/index`, {
      method: "POST",
      headers: auth(token),
    }),
  adminKnowledgeCandidates: (token: string, status?: string) =>
    request<AdminKnowledgeCandidate[]>(
      `/api/admin/knowledge/candidates${status ? `?status=${encodeURIComponent(status)}` : ""}`,
      { headers: auth(token) },
    ),
  adminKnowledgeCandidate: (token: string, id: string) =>
    request<AdminKnowledgeCandidateDetails>(`/api/admin/knowledge/candidates/${id}`, {
      headers: auth(token),
    }),
  adminApproveKnowledgeCandidate: (token: string, id: string) =>
    request<{ candidate: AdminKnowledgeCandidate }>(`/api/admin/knowledge/candidates/${id}/approve`, {
      method: "POST",
      headers: auth(token),
    }),
  adminRejectKnowledgeCandidate: (token: string, id: string, note = "") =>
    request<AdminKnowledgeCandidate>(`/api/admin/knowledge/candidates/${id}/reject`, {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify({ note }),
    }),
  adminAssistantProfiles: (token: string) =>
    request<AdminAssistantProfile[]>("/api/admin/assistant/profiles", { headers: auth(token) }),
  adminCreateAssistantProfile: (token: string, body: { name: string; slug: string; description?: string }) =>
    request<AdminAssistantProfile>("/api/admin/assistant/profiles", {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify(body),
    }),
  adminAssistantCandidates: (token: string) =>
    request<AdminAssistantCandidate[]>("/api/admin/assistant/candidates", { headers: auth(token) }),
  adminAssistantCandidate: (token: string, id: string) =>
    request<AdminAssistantCandidate>(`/api/admin/assistant/candidates/${id}`, { headers: auth(token) }),
  adminCreateAssistantCandidate: (
    token: string,
    body: {
      assistant_profile_id: string;
      candidate_type: string;
      title: string;
      description?: string;
      proposed_payload_json?: unknown;
      reason?: string;
      risk_level?: string;
    },
  ) =>
    request<AdminAssistantCandidate>("/api/admin/assistant/candidates", {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify(body),
    }),
  adminApproveAssistantCandidate: (token: string, id: string, note = "") =>
    request<AdminAssistantCandidate>(`/api/admin/assistant/candidates/${id}/approve`, {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify({ note }),
    }),
  adminRejectAssistantCandidate: (token: string, id: string, note = "") =>
    request<AdminAssistantCandidate>(`/api/admin/assistant/candidates/${id}/reject`, {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify({ note }),
    }),
  adminAssistantNotes: (token: string) =>
    request<AdminAssistantNote[]>("/api/admin/assistant/notes", { headers: auth(token) }),
  adminCreateAssistantNote: (token: string, body: { note_type: string; title: string; body?: string }) =>
    request<AdminAssistantNote>("/api/admin/assistant/notes", {
      method: "POST",
      headers: auth(token),
      body: JSON.stringify(body),
    }),
  adminConvertAssistantNote: (token: string, id: string) =>
    request<AdminAssistantCandidate>(`/api/admin/assistant/notes/${id}/convert-to-candidate`, {
      method: "POST",
      headers: auth(token),
    }),
};

function auth(token: string) {
  return { authorization: `Bearer ${token}` };
}
