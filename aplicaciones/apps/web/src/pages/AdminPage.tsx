import { useMutation, useQuery } from "@tanstack/react-query";
import { FormEvent, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { ErrorState, LoadingState } from "../components/ui/State";

export function AdminPage() {
  const [token, setToken] = useState(localStorage.getItem("admin-token") ?? "");
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const login = useMutation({
    mutationFn: () => api.adminLogin(username, password),
    onSuccess: (data) => {
      localStorage.setItem("admin-token", data.token);
      setToken(data.token);
    },
  });
  const stats = useQuery({ queryKey: ["admin-stats", token], queryFn: () => api.adminStats(token), enabled: Boolean(token) });
  const rag = useQuery({ queryKey: ["rag-status"], queryFn: api.ragStatus });
  const reindex = useMutation({ mutationFn: api.reindex, onSuccess: () => rag.refetch() });

  function submit(event: FormEvent) {
    event.preventDefault();
    login.mutate();
  }

  if (!token) {
    return (
      <section className="mx-auto max-w-md px-4 py-10">
        <h1 className="text-3xl font-semibold">Админ</h1>
        <form className="mt-6 space-y-3 rounded-md border border-ink/10 bg-white p-5" onSubmit={submit}>
          <input className="w-full rounded-md border border-ink/15 px-3 py-2" value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Логин" />
          <input className="w-full rounded-md border border-ink/15 px-3 py-2" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Пароль" type="password" />
          <button className="w-full rounded-md bg-brand px-4 py-2 font-medium text-white">Войти</button>
          {login.error && <ErrorState error={login.error} />}
        </form>
      </section>
    );
  }

  return (
    <section className="mx-auto max-w-6xl px-4 py-10">
      <h1 className="text-3xl font-semibold">Админ-панель</h1>
      <div className="mt-8 grid gap-4 md:grid-cols-4">
        {stats.isLoading && <LoadingState />}
        {stats.error && <ErrorState error={stats.error} />}
        {stats.data && Object.entries(stats.data.content).map(([key, value]) => (
          <div key={key} className="rounded-md border border-ink/10 bg-white p-5">
            <div className="text-sm text-ink/55">{key}</div>
            <div className="mt-2 text-3xl font-semibold">{value}</div>
          </div>
        ))}
      </div>
      <div className="mt-6 rounded-md border border-ink/10 bg-white p-5">
        <h2 className="text-xl font-semibold">RAG</h2>
        <p className="mt-2 text-sm text-ink/65">Коллекция: {rag.data?.collection ?? "..."}. Векторов: {rag.data?.vectors ?? 0}.</p>
        <button onClick={() => reindex.mutate()} className="mt-4 rounded-md bg-brand px-4 py-2 font-medium text-white disabled:opacity-50" disabled={reindex.isPending}>
          Переиндексировать docs
        </button>
        {reindex.data && <p className="mt-3 text-sm text-brand">Файлов: {reindex.data.files}, чанков: {reindex.data.chunks}</p>}
        {reindex.error && <div className="mt-3"><ErrorState error={reindex.error} /></div>}
      </div>
      <div className="mt-6 grid gap-3 md:grid-cols-2">
        <Link className="rounded-md border border-ink/10 bg-white p-5 hover:border-brand/40" to="/admin/knowledge">
          <h2 className="text-xl font-semibold">Knowledge intake</h2>
          <p className="mt-2 text-sm text-ink/65">Sources, facts, and draft candidates.</p>
        </Link>
        <Link className="rounded-md border border-ink/10 bg-white p-5 hover:border-brand/40" to="/admin/assistant">
          <h2 className="text-xl font-semibold">Assistant brain</h2>
          <p className="mt-2 text-sm text-ink/65">Profiles, policies, notes, and change candidates.</p>
        </Link>
      </div>
    </section>
  );
}
