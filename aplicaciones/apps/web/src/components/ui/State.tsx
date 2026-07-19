export function LoadingState({ label = "Загрузка..." }: { label?: string }) {
  return <div className="rounded-md border border-ink/10 bg-white p-4 text-sm text-ink/60">{label}</div>;
}

export function EmptyState({ label }: { label: string }) {
  return <div className="rounded-md border border-dashed border-ink/20 bg-white/70 p-6 text-sm text-ink/60">{label}</div>;
}

export function ErrorState({ error }: { error: unknown }) {
  return <div className="rounded-md border border-clay/30 bg-clay/10 p-4 text-sm text-clay">{error instanceof Error ? error.message : "Ошибка"}</div>;
}
