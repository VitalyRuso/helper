import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { ArticleCard } from "../components/cards/ArticleCard";
import { CategoryCard } from "../components/cards/CategoryCard";
import { EmptyState, ErrorState, LoadingState } from "../components/ui/State";

export function KbPage() {
  const categories = useQuery({ queryKey: ["categories"], queryFn: api.categories });
  const articles = useQuery({ queryKey: ["articles"], queryFn: api.articles });
  return (
    <section className="mx-auto max-w-7xl px-4 py-10">
      <h1 className="text-3xl font-semibold">База знаний</h1>
      <p className="mt-3 max-w-2xl text-ink/65">Категории и статьи для повседневных вопросов и административных процедур.</p>
      <h2 className="mt-8 text-xl font-semibold">Категории</h2>
      <div className="mt-4 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {categories.isLoading && <LoadingState />}
        {categories.error && <ErrorState error={categories.error} />}
        {categories.data?.map((category) => <CategoryCard key={category.id} category={category} />)}
      </div>
      <h2 className="mt-10 text-xl font-semibold">Статьи</h2>
      <div className="mt-4 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {articles.isLoading && <LoadingState />}
        {articles.error && <ErrorState error={articles.error} />}
        {articles.data?.length === 0 && <EmptyState label="Статей пока нет. Добавьте материалы через API или будущую CMS." />}
        {articles.data?.map((article) => <ArticleCard key={article.id} article={article} />)}
      </div>
    </section>
  );
}
