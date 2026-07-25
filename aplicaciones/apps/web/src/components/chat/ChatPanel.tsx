import { Send } from "lucide-react";
import { FormEvent, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { sessionId } from "../../lib/session";
import type { ChatResponse } from "../../types";

const JOB_POLL_MS = 2_000;
const JOB_WAIT_MS = 18 * 60_000;

function isLocalCommand(value: string) {
  const command = value.trim();
  return (
    command === "/help" ||
    command === "/status" ||
    (command.startsWith("/key ") && Boolean(command.slice(5).trim()))
  );
}

type Message = {
  role: "assistant" | "user";
  text: string;
  sources?: ChatResponse["sources"];
  legal?: ChatResponse["legal"];
};

export function ChatPanel({ compact = false }: { compact?: boolean }) {
  const [messages, setMessages] = useState<Message[]>([
    {
      role: "assistant",
      text: "Здравствуйте. Я помогу разобраться с инструкциями и документами по Испании. После индексации документов отвечаю только с источниками.",
    },
  ]);
  const [text, setText] = useState("");
  const statusQuery = useQuery({
    queryKey: ["local-llm-status"],
    queryFn: api.localLlmStatus,
    refetchInterval: 5_000,
    retry: false,
  });
  const localAiStatus = statusQuery.isError
    ? "offline"
    : (statusQuery.data?.status ?? "offline");
  const localCommand = isLocalCommand(text);
  const canSubmit =
    Boolean(text.trim()) &&
    (localAiStatus === "online" || localCommand);
  const submitTitle =
    localAiStatus === "online" || localCommand
      ? "Отправить"
      : localAiStatus === "busy"
        ? "Локальная нейронка занята"
        : "Локальная нейронка не подключена";
  const mutation = useMutation({
    mutationFn: async (message: string) => {
      const currentSessionId = sessionId();
      const submitted = await api.createLocalLlmJob(
        message,
        currentSessionId,
        location.pathname,
      );
      if (submitted.status === "completed" && submitted.response) {
        return submitted.response;
      }
      if (!submitted.job_id) {
        throw new Error("Не удалось создать запрос к локальной модели.");
      }

      const deadline = Date.now() + JOB_WAIT_MS;
      while (Date.now() < deadline) {
        await new Promise((resolve) => window.setTimeout(resolve, JOB_POLL_MS));
        const job = await api.localLlmJob(submitted.job_id, currentSessionId);
        if (job.status === "completed") {
          if (job.response) return job.response;
          throw new Error("Локальная модель вернула пустой ответ.");
        }
        if (job.status === "failed") {
          throw new Error(job.error || "Локальная модель не смогла обработать запрос.");
        }
        if (job.status === "expired") {
          throw new Error("Время обработки запроса истекло. Попробуйте ещё раз.");
        }
      }
      throw new Error("Локальная модель не ответила вовремя. Попробуйте ещё раз.");
    },
    onSuccess: (data) => {
      setMessages((items) => [...items, { role: "assistant", text: data.answer, sources: data.sources, legal: data.legal }]);
    },
    onError: (error) => {
      setMessages((items) => [...items, { role: "assistant", text: error instanceof Error ? error.message : "Ошибка запроса" }]);
    },
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    const message = text.trim();
    if (!message || mutation.isPending || !canSubmit) return;
    setMessages((items) => [...items, { role: "user", text: message }]);
    setText("");
    mutation.mutate(message);
  }

  return (
    <div className={`flex ${compact ? "h-[520px]" : "min-h-[640px]"} flex-col rounded-md border border-ink/10 bg-white shadow-soft`}>
      <div className="border-b border-ink/10 p-4">
        <div className="flex items-center justify-between gap-3">
          <div className="font-semibold">ИИ-помощник</div>
          <span
            aria-live="polite"
            className={`text-xs font-medium ${
              localAiStatus === "online"
                ? "text-emerald-700"
                : localAiStatus === "busy"
                  ? "text-saffron"
                  : "text-ink/50"
            }`}
          >
            {localAiStatus === "online"
              ? "Local AI Online"
              : localAiStatus === "busy"
                ? "Local AI Busy"
                : "Local AI Offline"}
          </span>
        </div>
        <div className="text-xs text-ink/60">Команды: /help, /status, /key ACCESS_KEY</div>
        {localAiStatus === "offline" && (
          <div className="mt-1 text-xs text-ink/60">Локальная нейронка не подключена</div>
        )}
      </div>
      <div className="flex-1 space-y-3 overflow-auto p-4">
        {messages.map((message, index) => (
          <div key={index} className={message.role === "user" ? "text-right" : "text-left"}>
            <div className={`inline-block max-w-[88%] whitespace-pre-wrap rounded-md px-4 py-3 text-sm leading-6 ${message.role === "user" ? "bg-brand text-white" : "bg-paper text-ink"}`}>
              {message.text}
              {message.sources && message.sources.length > 0 && (
                <div className="mt-3 border-t border-ink/10 pt-2 text-left text-xs text-ink/65">
                  Источники: {Array.from(new Set(message.sources.map((source) => source.file_name))).join(", ")}
                </div>
              )}
              {message.legal && (
                <div className="mt-3 border-t border-ink/10 pt-2 text-left text-xs leading-5 text-ink/65">
                  <p className="font-medium text-ink">
                    Legal Reviewer · {message.legal.reviewed ? "Reviewed" : "Not reviewed"} ·{" "}
                    {message.legal.currentness.reviewed_version_is_current
                      ? "Current"
                      : `Currentness: ${message.legal.currentness.status || "unknown"}`}
                  </p>
                  {(message.legal.legal_area || message.legal.procedure_key) && (
                    <p>{[message.legal.legal_area, message.legal.procedure_key].filter(Boolean).join(" · ")}</p>
                  )}
                  <p>Last checked: {message.legal.currentness.last_checked_at ?? "unknown"}</p>
                  {message.legal.sources.length > 0 && (
                    <ul className="mt-1 space-y-1">
                      {message.legal.sources.map((source) => {
                        const label = [
                          source.document_title,
                          source.authority,
                          source.official_id,
                          source.eli_id,
                          source.version_label,
                          source.version_date,
                        ]
                          .filter(Boolean)
                          .join(" · ");
                        const href = /^https?:\/\//i.test(source.source_url ?? "") ? source.source_url : null;
                        return (
                          <li key={source.knowledge_item_id}>
                            {href ? (
                              <a className="text-brand underline" href={href} target="_blank" rel="noreferrer">
                                {label || href}
                              </a>
                            ) : (
                              label || "Source details unavailable"
                            )}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                  {!message.text.includes(message.legal.disclaimer) && <p className="mt-1">{message.legal.disclaimer}</p>}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
      <form className="flex gap-2 border-t border-ink/10 p-3" onSubmit={submit}>
        <input
          value={text}
          onChange={(event) => setText(event.target.value)}
          className="min-w-0 flex-1 rounded-md border border-ink/15 px-3 py-2 text-sm outline-none focus:border-brand"
          placeholder="Спросите о процедуре..."
        />
        <button
          className="grid h-10 w-10 place-items-center rounded-md bg-brand text-white disabled:opacity-50"
          disabled={mutation.isPending || !canSubmit}
          title={submitTitle}
          aria-label={submitTitle}
        >
          <Send className="h-4 w-4" />
        </button>
      </form>
    </div>
  );
}
