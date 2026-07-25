import { useQuery } from "@tanstack/react-query";
import { Bot, Search } from "lucide-react";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { CategoryCard } from "../components/cards/CategoryCard";
import { GuideCard } from "../components/cards/GuideCard";
import { EmptyState, ErrorState, LoadingState } from "../components/ui/State";

export function HomePage() {
  const categories = useQuery({ queryKey: ["categories"], queryFn: api.categories });
  const guides = useQuery({ queryKey: ["guides"], queryFn: api.guides });
  return (
    <>
      <section className="relative min-h-[520px] overflow-hidden bg-ink text-white">
        <img src="/hero-spain-helper.png" alt="" className="absolute inset-0 h-full w-full object-cover opacity-55" />
        <div className="absolute inset-0 bg-gradient-to-r from-ink via-ink/70 to-transparent" />
        <div className="relative mx-auto flex min-h-[520px] max-w-7xl flex-col justify-center px-4 py-16">
          <div className="max-w-2xl">
            <h1 className="text-4xl font-semibold leading-tight md:text-6xl">Spain Helper AI</h1>
            <p className="mt-5 text-lg leading-8 text-white/85">
              Русскоязычный портал по жизни и административным процедурам в Испании: гайды, база знаний и ИИ-помощник с источниками.
            </p>
            <div className="mt-8 flex flex-col gap-3 sm:flex-row">
              <Link to="/assistant" className="inline-flex items-center justify-center gap-2 rounded-md bg-brand px-5 py-3 font-medium">
                <Bot className="h-5 w-5" /> Открыть помощника
              </Link>
              <Link to="/search" className="inline-flex items-center justify-center gap-2 rounded-md bg-white px-5 py-3 font-medium text-ink">
                <Search className="h-5 w-5" /> Найти инструкцию
              </Link>
            </div>
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-4 py-12">
        <div className="mb-6 flex items-end justify-between">
          <div>
            <h2 className="text-2xl font-semibold">Основные разделы</h2>
            <p className="mt-2 text-ink/65">Темы для первых недель и сложных процедур.</p>
          </div>
          <Link to="/kb" className="text-sm font-medium text-brand">Вся база</Link>
        </div>
        {categories.isLoading && <LoadingState />}
        {categories.error && <ErrorState error={categories.error} />}
        {categories.data && categories.data.length === 0 && <EmptyState label="Категории пока не добавлены." />}
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {categories.data?.slice(0, 8).map((category) => <CategoryCard key={category.id} category={category} />)}
        </div>
      </section>

      <section className="bg-white/55 py-12">
        <div className="mx-auto max-w-7xl px-4">
          <h2 className="text-2xl font-semibold">Популярные гайды</h2>
          <div className="mt-6 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {guides.isLoading && <LoadingState />}
            {guides.error && <ErrorState error={guides.error} />}
            {guides.data?.length === 0 && <EmptyState label="Гайды пока не добавлены." />}
            {guides.data?.slice(0, 6).map((guide) => <GuideCard key={guide.id} guide={guide} />)}
          </div>
        </div>
      </section>
    </>
  );
}
