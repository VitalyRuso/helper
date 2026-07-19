import { ArrowRight } from "lucide-react";
import { Link } from "react-router-dom";
import type { Guide } from "../../types";

export function GuideCard({ guide }: { guide: Guide }) {
  return (
    <Link to={`/guides/${guide.slug}`} className="group rounded-md border border-ink/10 bg-white p-5 shadow-sm">
      <h3 className="text-lg font-semibold">{guide.title_ru}</h3>
      <p className="mt-2 min-h-12 text-sm leading-6 text-ink/65">{guide.summary_ru}</p>
      <span className="mt-4 inline-flex items-center gap-2 text-sm font-medium text-brand">
        Открыть <ArrowRight className="h-4 w-4 transition group-hover:translate-x-1" />
      </span>
    </Link>
  );
}
