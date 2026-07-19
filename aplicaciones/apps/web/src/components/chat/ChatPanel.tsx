import { Send } from "lucide-react";
import { FormEvent, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../api/client";
import { sessionId } from "../../lib/session";

type Message = {
  role: "assistant" | "user";
  text: string;
};

export function ChatPanel({ compact = false }: { compact?: boolean }) {
  const [messages, setMessages] = useState<Message[]>([
    {
      role: "assistant",
      text: "Здравствуйте. Я помогу разобраться с инструкциями и документами по Испании. После индексации документов отвечаю только с источниками.",
    },
  ]);
  const [text, setText] = useState("");
  const mutation = useMutation({
    mutationFn: (message: string) => api.chat(message, sessionId(), location.pathname),
    onSuccess: (data) => {
      setMessages((items) => [...items, { role: "assistant", text: data.answer }]);
    },
    onError: (error) => {
      setMessages((items) => [...items, { role: "assistant", text: error instanceof Error ? error.message : "Ошибка запроса" }]);
    },
  });

  function submit(event: FormEvent) {
    event.preventDefault();
    const message = text.trim();
    if (!message || mutation.isPending) return;
    setMessages((items) => [...items, { role: "user", text: message }]);
    setText("");
    mutation.mutate(message);
  }

  return (
    <div className={`flex ${compact ? "h-[520px]" : "min-h-[640px]"} flex-col rounded-md border border-ink/10 bg-white shadow-soft`}>
      <div className="border-b border-ink/10 p-4">
        <div className="font-semibold">ИИ-помощник</div>
        <div className="text-xs text-ink/60">Команды: /help, /status, /key ACCESS_KEY</div>
      </div>
      <div className="flex-1 space-y-3 overflow-auto p-4">
        {messages.map((message, index) => (
          <div key={index} className={message.role === "user" ? "text-right" : "text-left"}>
            <div className={`inline-block max-w-[88%] whitespace-pre-wrap rounded-md px-4 py-3 text-sm leading-6 ${message.role === "user" ? "bg-brand text-white" : "bg-paper text-ink"}`}>
              {message.text}
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
        <button className="grid h-10 w-10 place-items-center rounded-md bg-brand text-white disabled:opacity-50" disabled={mutation.isPending} title="Отправить">
          <Send className="h-4 w-4" />
        </button>
      </form>
    </div>
  );
}
