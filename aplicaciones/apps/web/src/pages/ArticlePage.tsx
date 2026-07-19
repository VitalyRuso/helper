import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { api } from "../api/client";
import { ErrorState, LoadingState } from "../components/ui/State";

export function ArticlePage() {
  const { slug = "" } = useParams();
  const article = useQuery({ queryKey: ["article", slug], queryFn: () => api.article(slug) });
  return (
    <section className="mx-auto max-w-3xl px-4 py-10">
      {article.isLoading && <LoadingState />}
      {article.error && <ErrorState error={article.error} />}
      {article.data && (
        <>
          <div className="text-sm font-medium uppercase text-clay">{article.data.legal_risk_level}</div>
          <h1 className="mt-2 text-3xl font-semibold">{article.data.title_ru}</h1>
          <p className="mt-3 text-lg text-ink/65">{article.data.summary_ru}</p>
          <article className="prose-lite mt-8 whitespace-pre-wrap rounded-md border border-ink/10 bg-white p-6 leading-7">
            {article.data.body_ru_markdown}
          </article>
        </>
      )}
    </section>
  );
}
