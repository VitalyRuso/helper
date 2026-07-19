import { Link } from "react-router-dom";
import type { Category } from "../../types";

export function CategoryCard({ category }: { category: Category }) {
  return (
    <Link to={`/kb?category=${category.slug}`} className="rounded-md border border-ink/10 bg-white p-5 shadow-sm transition hover:-translate-y-0.5 hover:shadow-soft">
      <div className="text-xs uppercase tracking-wide text-brand">{category.icon}</div>
      <h3 className="mt-3 text-lg font-semibold">{category.title_ru}</h3>
      <p className="mt-2 text-sm leading-6 text-ink/65">{category.description_ru}</p>
    </Link>
  );
}
