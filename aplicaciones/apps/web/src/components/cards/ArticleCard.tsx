import { ArrowRight } from "lucide-react";
import { Link } from "react-router-dom";
import type { Article } from "../../types";

export function ArticleCard({ article }: { article: Article }) {
  return (
    <Link to={`/kb/${article.slug}`} className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
      <div className="text-xs font-medium uppercase text-clay">{article.legal_risk_level}</div>
      <h3 className="mt-2 text-lg font-semibold">{article.title_ru}</h3>
      <p className="mt-2 text-sm leading-6 text-ink/65">{article.summary_ru}</p>
      <span className="mt-4 inline-flex items-center gap-2 text-sm font-medium text-brand">
        Читать <ArrowRight className="h-4 w-4" />
      </span>
    </Link>
  );
}
