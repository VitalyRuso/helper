import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { GuideCard } from "../components/cards/GuideCard";
import { EmptyState, ErrorState, LoadingState } from "../components/ui/State";

export function GuidesPage() {
  const guides = useQuery({ queryKey: ["guides"], queryFn: api.guides });
  return (
    <section className="mx-auto max-w-7xl px-4 py-10">
      <h1 className="text-3xl font-semibold">Гайды</h1>
      <p className="mt-3 max-w-2xl text-ink/65">Структурированные инструкции: подготовка, документы, риски и официальные источники, когда они добавлены.</p>
      <div className="mt-8 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {guides.isLoading && <LoadingState />}
        {guides.error && <ErrorState error={guides.error} />}
        {guides.data?.length === 0 && <EmptyState label="Гайды пока не опубликованы." />}
        {guides.data?.map((guide) => <GuideCard key={guide.id} guide={guide} />)}
      </div>
    </section>
  );
}
