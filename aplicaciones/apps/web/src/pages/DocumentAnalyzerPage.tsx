import { FileSearch } from "lucide-react";
import { Link } from "react-router-dom";

export function DocumentAnalyzerPage() {
  return (
    <section className="mx-auto max-w-4xl px-4 py-10">
      <div className="rounded-md border border-ink/10 bg-white p-8 shadow-sm">
        <FileSearch className="h-10 w-10 text-brand" />
        <h1 className="mt-5 text-3xl font-semibold">Анализ документов</h1>
        <p className="mt-4 text-lg leading-8 text-ink/70">
          Модуль анализа документов готовится. Сейчас можно использовать ИИ-помощника и базу инструкций.
        </p>
        <Link to="/assistant" className="mt-6 inline-flex rounded-md bg-brand px-5 py-3 font-medium text-white">Открыть помощника</Link>
      </div>
    </section>
  );
}
