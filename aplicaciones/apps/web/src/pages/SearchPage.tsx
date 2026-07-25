import { useQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { FormEvent, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { EmptyState, ErrorState, LoadingState } from "../components/ui/State";

export function SearchPage() {
  const [input, setInput] = useState("");
  const [q, setQ] = useState("");
  const results = useQuery({ queryKey: ["search", q], queryFn: () => api.search(q), enabled: Boolean(q) });
  function submit(event: FormEvent) {
    event.preventDefault();
    setQ(input.trim());
  }
  return (
    <section className="mx-auto max-w-4xl px-4 py-10">
      <h1 className="text-3xl font-semibold">Поиск</h1>
      <form onSubmit={submit} className="mt-6 flex gap-2">
        <input className="min-w-0 flex-1 rounded-md border border-ink/15 px-4 py-3 outline-none focus:border-brand" value={input} onChange={(e) => setInput(e.target.value)} placeholder="Например: cita previa" />
        <button className="grid h-12 w-12 place-items-center rounded-md bg-brand text-white" title="Искать"><Search className="h-5 w-5" /></button>
      </form>
      <div className="mt-8 space-y-3">
        {results.isLoading && <LoadingState />}
        {results.error && <ErrorState error={results.error} />}
        {q && results.data?.length === 0 && <EmptyState label="Ничего не найдено." />}
        {results.data?.map((item) => (
          <Link key={`${item.kind}-${item.slug}`} to={item.kind === "guide" ? `/guides/${item.slug}` : `/kb/${item.slug}`} className="block rounded-md border border-ink/10 bg-white p-5">
            <div className="text-xs uppercase text-brand">{item.kind}</div>
            <h2 className="mt-1 font-semibold">{item.title_ru}</h2>
            <p className="mt-2 text-sm text-ink/65">{item.summary_ru}</p>
          </Link>
        ))}
      </div>
    </section>
  );
}
