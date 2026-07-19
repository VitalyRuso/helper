import { ChatPanel } from "../components/chat/ChatPanel";

export function AssistantPage() {
  return (
    <section className="mx-auto grid max-w-7xl gap-8 px-4 py-10 lg:grid-cols-[1fr_420px]">
      <div>
        <h1 className="text-3xl font-semibold">ИИ-помощник</h1>
        <p className="mt-3 max-w-2xl text-ink/65">
          Помощник отвечает на русском и использует только проиндексированные материалы. Если документов нет, он скажет, что данных недостаточно.
        </p>
        <div className="mt-8 rounded-md border border-saffron/30 bg-saffron/10 p-5 text-sm leading-6 text-ink/75">
          Это информационный инструмент. Он не заменяет юриста, gestor, Extranjería или официальный источник.
        </div>
      </div>
      <ChatPanel />
    </section>
  );
}
