import { KeyRound } from "lucide-react";

export function PricingPage() {
  return (
    <section className="mx-auto max-w-5xl px-4 py-10">
      <h1 className="text-3xl font-semibold">Доступ</h1>
      <div className="mt-8 grid gap-4 md:grid-cols-2">
        <div className="rounded-md border border-ink/10 bg-white p-6">
          <h2 className="text-xl font-semibold">Гостевой доступ</h2>
          <p className="mt-3 text-ink/65">3 вопроса к ИИ-помощнику. База знаний и гайды доступны публично.</p>
        </div>
        <div className="rounded-md border border-brand/30 bg-white p-6">
          <KeyRound className="h-8 w-8 text-brand" />
          <h2 className="mt-4 text-xl font-semibold">Ключ доступа</h2>
          <p className="mt-3 text-ink/65">Введите команду /key ACCESS_KEY в чате. Stripe и подписки можно добавить позже через существующую модель сессий.</p>
        </div>
      </div>
    </section>
  );
}
