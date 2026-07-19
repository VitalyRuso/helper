import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { api } from "../api/client";
import { ErrorState, LoadingState } from "../components/ui/State";

export function GuideDetailPage() {
  const { slug = "" } = useParams();
  const guide = useQuery({ queryKey: ["guide", slug], queryFn: () => api.guide(slug) });
  if (guide.isLoading) return <Page><LoadingState /></Page>;
  if (guide.error) return <Page><ErrorState error={guide.error} /></Page>;
  if (!guide.data) return null;
  return (
    <Page>
      <h1 className="text-3xl font-semibold">{guide.data.title_ru}</h1>
      <p className="mt-3 text-lg text-ink/65">{guide.data.summary_ru}</p>
      <div className="mt-8 grid gap-4 lg:grid-cols-2">
        <Block title="Для кого">{guide.data.target_audience}</Block>
        <Block title="Куда подавать">{guide.data.where_to_submit}</Block>
        <List title="Документы" items={guide.data.required_documents} />
        <List title="Шаги" items={guide.data.steps} />
        <List title="Сроки" items={guide.data.deadlines} />
        <List title="Риски" items={guide.data.risks} />
      </div>
    </Page>
  );
}

function Page({ children }: { children: React.ReactNode }) {
  return <section className="mx-auto max-w-5xl px-4 py-10">{children}</section>;
}

function Block({ title, children }: { title: string; children: React.ReactNode }) {
  return <div className="rounded-md border border-ink/10 bg-white p-5"><h2 className="font-semibold">{title}</h2><div className="mt-3 text-sm leading-6 text-ink/70">{children}</div></div>;
}

function List({ title, items }: { title: string; items: string[] }) {
  return <Block title={title}><ul className="list-disc space-y-2 pl-5">{items.map((item) => <li key={item}>{item}</li>)}</ul></Block>;
}
