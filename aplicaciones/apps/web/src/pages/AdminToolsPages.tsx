import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, FileSearch, Play, Plus, X } from "lucide-react";
import { FormEvent, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api } from "../api/client";
import { EmptyState, ErrorState, LoadingState } from "../components/ui/State";
import type { AdminAssistantProfile, AdminKnowledgeCandidate } from "../types";

function token() {
  return localStorage.getItem("admin-token") ?? "";
}

function NeedLogin() {
  return (
    <section className="mx-auto max-w-3xl px-4 py-10">
      <h1 className="text-2xl font-semibold">Admin</h1>
      <Link className="mt-4 inline-flex rounded-md bg-brand px-4 py-2 font-medium text-white" to="/admin">
        Login
      </Link>
    </section>
  );
}

function Shell({ title, children }: { title: string; children: React.ReactNode }) {
  const links = [
    ["/admin/legal", "Legal Review"],
    ["/admin/knowledge", "Knowledge"],
    ["/admin/knowledge/sources", "Sources"],
    ["/admin/knowledge/candidates", "Candidates"],
    ["/admin/assistant", "Assistant"],
    ["/admin/assistant/profiles", "Profiles"],
    ["/admin/assistant/candidates", "Assistant candidates"],
    ["/admin/assistant/notes", "Notes"],
  ];
  return (
    <section className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-3xl font-semibold">{title}</h1>
      <nav className="mt-5 flex flex-wrap gap-2 text-sm">
        {links.map(([to, label]) => (
          <Link key={to} to={to} className="rounded-md border border-ink/10 bg-white px-3 py-2 hover:border-brand/40">
            {label}
          </Link>
        ))}
      </nav>
      <div className="mt-6">{children}</div>
    </section>
  );
}

export function Guard({ title, children }: { title: string; children: React.ReactNode }) {
  if (!token()) return <NeedLogin />;
  return <Shell title={title}>{children}</Shell>;
}

export function Badge({ children }: { children: React.ReactNode }) {
  return <span className="rounded-md border border-ink/10 bg-white px-2 py-1 text-xs text-ink/65">{children}</span>;
}

export function ActionButton({
  children,
  onClick,
  disabled,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      className="inline-flex items-center gap-2 rounded-md border border-ink/10 bg-white px-3 py-2 text-sm font-medium hover:border-brand/40 disabled:opacity-50"
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export function AdminKnowledgePage() {
  const t = token();
  const sources = useQuery({ queryKey: ["admin-knowledge-sources"], queryFn: () => api.adminKnowledgeSources(t) });
  const candidates = useQuery({ queryKey: ["admin-knowledge-candidates"], queryFn: () => api.adminKnowledgeCandidates(t) });
  const indexed = sources.data?.filter((source) => source.status === "indexed").length ?? 0;
  const drafts = candidates.data?.filter((candidate) => candidate.status === "draft").length ?? 0;
  return (
    <Guard title="Knowledge intake">
      <div className="grid gap-4 md:grid-cols-3">
        <Metric label="Sources" value={sources.data?.length ?? 0} />
        <Metric label="Draft candidates" value={drafts} />
        <Metric label="Indexed sources" value={indexed} />
      </div>
      {(sources.error || candidates.error) && <div className="mt-4"><ErrorState error={sources.error ?? candidates.error} /></div>}
    </Guard>
  );
}

export function AdminKnowledgeSourcesPage() {
  const t = token();
  const qc = useQueryClient();
  const [title, setTitle] = useState("");
  const [rawText, setRawText] = useState("");
  const sources = useQuery({ queryKey: ["admin-knowledge-sources"], queryFn: () => api.adminKnowledgeSources(t) });
  const scan = useMutation({ mutationFn: () => api.adminScanDocs(t), onSuccess: () => qc.invalidateQueries({ queryKey: ["admin-knowledge-sources"] }) });
  const create = useMutation({
    mutationFn: () =>
      api.adminCreateKnowledgeSource(t, {
        title: title || "Pasted text",
        source_type: "pasted_text",
        raw_text: rawText,
        trust_level: "user_provided",
      }),
    onSuccess: () => {
      setTitle("");
      setRawText("");
      qc.invalidateQueries({ queryKey: ["admin-knowledge-sources"] });
    },
  });
  const analyze = useMutation({ mutationFn: (id: string) => api.adminAnalyzeSource(t, id), onSuccess: () => qc.invalidateQueries() });
  const index = useMutation({ mutationFn: (id: string) => api.adminIndexSource(t, id), onSuccess: () => qc.invalidateQueries() });

  function submit(event: FormEvent) {
    event.preventDefault();
    create.mutate();
  }

  return (
    <Guard title="Knowledge sources">
      <div className="flex flex-wrap gap-2">
        <ActionButton onClick={() => scan.mutate()} disabled={scan.isPending}><FileSearch className="h-4 w-4" /> Scan docs</ActionButton>
      </div>
      <form className="mt-4 grid gap-3 rounded-md border border-ink/10 bg-white p-4" onSubmit={submit}>
        <input className="rounded-md border border-ink/15 px-3 py-2" value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Title" />
        <textarea className="min-h-28 rounded-md border border-ink/15 px-3 py-2" value={rawText} onChange={(event) => setRawText(event.target.value)} placeholder="Pasted text" />
        <button className="inline-flex w-fit items-center gap-2 rounded-md bg-brand px-4 py-2 font-medium text-white" disabled={!rawText.trim() || create.isPending}>
          <Plus className="h-4 w-4" /> Add pasted text
        </button>
      </form>
      <ListState loading={sources.isLoading} error={sources.error} empty={!sources.data?.length} />
      <div className="mt-4 grid gap-3">
        {sources.data?.map((source) => (
          <div key={source.id} className="rounded-md border border-ink/10 bg-white p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 className="font-semibold">{source.title}</h2>
                <div className="mt-2 flex flex-wrap gap-2"><Badge>{source.source_type}</Badge><Badge>{source.status}</Badge><Badge>{source.trust_level}</Badge></div>
                {source.original_path && <p className="mt-2 break-all text-sm text-ink/55">{source.original_path}</p>}
              </div>
              <div className="flex gap-2">
                <ActionButton onClick={() => analyze.mutate(source.id)} disabled={analyze.isPending}><Play className="h-4 w-4" /> Analyze</ActionButton>
                <ActionButton onClick={() => index.mutate(source.id)} disabled={index.isPending}><FileSearch className="h-4 w-4" /> Index</ActionButton>
              </div>
            </div>
          </div>
        ))}
      </div>
    </Guard>
  );
}

export function AdminKnowledgeCandidatesPage() {
  const t = token();
  const qc = useQueryClient();
  const [status, setStatus] = useState("draft");
  const candidates = useQuery({ queryKey: ["admin-knowledge-candidates", status], queryFn: () => api.adminKnowledgeCandidates(t, status || undefined) });
  const approve = useMutation({ mutationFn: (id: string) => api.adminApproveKnowledgeCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  const reject = useMutation({ mutationFn: (id: string) => api.adminRejectKnowledgeCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  return (
    <Guard title="Knowledge candidates">
      <select className="rounded-md border border-ink/15 bg-white px-3 py-2" value={status} onChange={(event) => setStatus(event.target.value)}>
        <option value="">all</option>
        <option value="draft">draft</option>
        <option value="approved">approved</option>
        <option value="rejected">rejected</option>
      </select>
      <CandidateList candidates={candidates.data} loading={candidates.isLoading} error={candidates.error} approve={approve.mutate} reject={reject.mutate} />
    </Guard>
  );
}

export function AdminKnowledgeCandidateDetailPage() {
  const t = token();
  const { id = "" } = useParams();
  const qc = useQueryClient();
  const details = useQuery({ queryKey: ["admin-knowledge-candidate", id], queryFn: () => api.adminKnowledgeCandidate(t, id), enabled: Boolean(id) });
  const approve = useMutation({ mutationFn: () => api.adminApproveKnowledgeCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  const reject = useMutation({ mutationFn: () => api.adminRejectKnowledgeCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  return (
    <Guard title="Candidate preview">
      <ListState loading={details.isLoading} error={details.error} empty={!details.data} />
      {details.data && (
        <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
          <article className="rounded-md border border-ink/10 bg-white p-5">
            <div className="flex flex-wrap gap-2"><Badge>{details.data.candidate.status}</Badge><Badge>{details.data.candidate.risk_level}</Badge></div>
            <h2 className="mt-3 text-2xl font-semibold">{details.data.candidate.title_ru}</h2>
            <p className="mt-2 text-ink/65">{details.data.candidate.summary_ru}</p>
            <pre className="mt-5 whitespace-pre-wrap rounded-md bg-paper p-4 text-sm">{details.data.candidate.body_ru_markdown}</pre>
            <div className="mt-4 flex gap-2">
              <ActionButton onClick={() => approve.mutate()} disabled={approve.isPending}><Check className="h-4 w-4" /> Approve as draft</ActionButton>
              <ActionButton onClick={() => reject.mutate()} disabled={reject.isPending}><X className="h-4 w-4" /> Reject</ActionButton>
            </div>
          </article>
          <aside className="space-y-4">
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <h3 className="font-semibold">Source</h3>
              <p className="mt-2 text-sm text-ink/65">{details.data.source?.title ?? "No source"}</p>
            </div>
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <h3 className="font-semibold">Facts</h3>
              <div className="mt-3 space-y-2">
                {details.data.facts.map((fact) => <p key={fact.id} className="text-sm text-ink/70">{fact.text_ru}</p>)}
              </div>
            </div>
          </aside>
        </div>
      )}
    </Guard>
  );
}

export function AdminAssistantPage() {
  const t = token();
  const profiles = useQuery({ queryKey: ["admin-assistant-profiles"], queryFn: () => api.adminAssistantProfiles(t) });
  const candidates = useQuery({ queryKey: ["admin-assistant-candidates"], queryFn: () => api.adminAssistantCandidates(t) });
  const active = profiles.data?.find((profile) => profile.is_active);
  return (
    <Guard title="Assistant brain">
      <div className="grid gap-4 md:grid-cols-4">
        <Metric label="Active profile" value={active?.name ?? "none"} />
        <Metric label="Active prompt" value={active?.active_prompt_version_id ? "set" : "fallback"} />
        <Metric label="Active policy" value={active?.active_policy_version_id ? "set" : "fallback"} />
        <Metric label="Pending candidates" value={candidates.data?.filter((candidate) => candidate.status === "draft").length ?? 0} />
      </div>
    </Guard>
  );
}

export function AdminAssistantProfilesPage() {
  const t = token();
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const profiles = useQuery({ queryKey: ["admin-assistant-profiles"], queryFn: () => api.adminAssistantProfiles(t) });
  const create = useMutation({
    mutationFn: () => api.adminCreateAssistantProfile(t, { name, slug, description: "" }),
    onSuccess: () => {
      setName("");
      setSlug("");
      qc.invalidateQueries({ queryKey: ["admin-assistant-profiles"] });
    },
  });
  return (
    <Guard title="Assistant profiles">
      <form className="grid gap-3 rounded-md border border-ink/10 bg-white p-4 md:grid-cols-[1fr_1fr_auto]" onSubmit={(event) => { event.preventDefault(); create.mutate(); }}>
        <input className="rounded-md border border-ink/15 px-3 py-2" value={name} onChange={(event) => setName(event.target.value)} placeholder="Name" />
        <input className="rounded-md border border-ink/15 px-3 py-2" value={slug} onChange={(event) => setSlug(event.target.value)} placeholder="slug" />
        <button className="inline-flex items-center gap-2 rounded-md bg-brand px-4 py-2 font-medium text-white" disabled={!name || !slug || create.isPending}><Plus className="h-4 w-4" /> Create</button>
      </form>
      <ListState loading={profiles.isLoading} error={profiles.error} empty={!profiles.data?.length} />
      <div className="mt-4 grid gap-3">
        {profiles.data?.map((profile) => <ProfileCard key={profile.id} profile={profile} />)}
      </div>
    </Guard>
  );
}

export function AdminAssistantCandidatesPage() {
  const t = token();
  const qc = useQueryClient();
  const [title, setTitle] = useState("");
  const profiles = useQuery({ queryKey: ["admin-assistant-profiles"], queryFn: () => api.adminAssistantProfiles(t) });
  const candidates = useQuery({ queryKey: ["admin-assistant-candidates"], queryFn: () => api.adminAssistantCandidates(t) });
  const profileId = profiles.data?.[0]?.id ?? "";
  const create = useMutation({
    mutationFn: () =>
      api.adminCreateAssistantCandidate(t, {
        assistant_profile_id: profileId,
        candidate_type: "architecture_note",
        title,
        proposed_payload_json: { note: title },
        risk_level: "low",
      }),
    onSuccess: () => {
      setTitle("");
      qc.invalidateQueries({ queryKey: ["admin-assistant-candidates"] });
    },
  });
  const approve = useMutation({ mutationFn: (id: string) => api.adminApproveAssistantCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  const reject = useMutation({ mutationFn: (id: string) => api.adminRejectAssistantCandidate(t, id), onSuccess: () => qc.invalidateQueries() });
  return (
    <Guard title="Assistant candidates">
      <form className="flex flex-wrap gap-3 rounded-md border border-ink/10 bg-white p-4" onSubmit={(event) => { event.preventDefault(); create.mutate(); }}>
        <input className="min-w-64 flex-1 rounded-md border border-ink/15 px-3 py-2" value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Architecture note" />
        <button className="inline-flex items-center gap-2 rounded-md bg-brand px-4 py-2 font-medium text-white" disabled={!profileId || !title || create.isPending}><Plus className="h-4 w-4" /> Create</button>
      </form>
      <div className="mt-4 grid gap-3">
        {candidates.data?.map((candidate) => (
          <div key={candidate.id} className="rounded-md border border-ink/10 bg-white p-4">
            <div className="flex flex-wrap justify-between gap-3">
              <div>
                <Link className="font-semibold text-brand" to={`/admin/assistant/candidates/${candidate.id}`}>{candidate.title}</Link>
                <div className="mt-2 flex flex-wrap gap-2"><Badge>{candidate.candidate_type}</Badge><Badge>{candidate.status}</Badge><Badge>{candidate.risk_level}</Badge></div>
              </div>
              <div className="flex gap-2">
                <ActionButton onClick={() => approve.mutate(candidate.id)} disabled={approve.isPending}><Check className="h-4 w-4" /> Approve</ActionButton>
                <ActionButton onClick={() => reject.mutate(candidate.id)} disabled={reject.isPending}><X className="h-4 w-4" /> Reject</ActionButton>
              </div>
            </div>
          </div>
        ))}
      </div>
      <ListState loading={candidates.isLoading} error={candidates.error} empty={!candidates.data?.length} />
    </Guard>
  );
}

export function AdminAssistantCandidateDetailPage() {
  const t = token();
  const { id = "" } = useParams();
  const candidate = useQuery({ queryKey: ["admin-assistant-candidate", id], queryFn: () => api.adminAssistantCandidate(t, id), enabled: Boolean(id) });
  return (
    <Guard title="Assistant candidate">
      <ListState loading={candidate.isLoading} error={candidate.error} empty={!candidate.data} />
      {candidate.data && (
        <article className="rounded-md border border-ink/10 bg-white p-5">
          <div className="flex flex-wrap gap-2"><Badge>{candidate.data.candidate_type}</Badge><Badge>{candidate.data.status}</Badge><Badge>{candidate.data.risk_level}</Badge></div>
          <h2 className="mt-3 text-2xl font-semibold">{candidate.data.title}</h2>
          <p className="mt-2 text-ink/65">{candidate.data.description}</p>
          <pre className="mt-4 whitespace-pre-wrap rounded-md bg-paper p-4 text-sm">{JSON.stringify(candidate.data.proposed_payload_json, null, 2)}</pre>
        </article>
      )}
    </Guard>
  );
}

export function AdminAssistantNotesPage() {
  const t = token();
  const qc = useQueryClient();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const notes = useQuery({ queryKey: ["admin-assistant-notes"], queryFn: () => api.adminAssistantNotes(t) });
  const create = useMutation({
    mutationFn: () => api.adminCreateAssistantNote(t, { note_type: "idea", title, body }),
    onSuccess: () => {
      setTitle("");
      setBody("");
      qc.invalidateQueries({ queryKey: ["admin-assistant-notes"] });
    },
  });
  const convert = useMutation({ mutationFn: (id: string) => api.adminConvertAssistantNote(t, id), onSuccess: () => qc.invalidateQueries() });
  return (
    <Guard title="Assistant notes">
      <form className="grid gap-3 rounded-md border border-ink/10 bg-white p-4" onSubmit={(event) => { event.preventDefault(); create.mutate(); }}>
        <input className="rounded-md border border-ink/15 px-3 py-2" value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Title" />
        <textarea className="min-h-24 rounded-md border border-ink/15 px-3 py-2" value={body} onChange={(event) => setBody(event.target.value)} placeholder="Note" />
        <button className="inline-flex w-fit items-center gap-2 rounded-md bg-brand px-4 py-2 font-medium text-white" disabled={!title || create.isPending}><Plus className="h-4 w-4" /> Add note</button>
      </form>
      <div className="mt-4 grid gap-3">
        {notes.data?.map((note) => (
          <div key={note.id} className="rounded-md border border-ink/10 bg-white p-4">
            <div className="flex flex-wrap justify-between gap-3">
              <div>
                <h2 className="font-semibold">{note.title}</h2>
                <p className="mt-1 text-sm text-ink/65">{note.body}</p>
                <div className="mt-2 flex flex-wrap gap-2"><Badge>{note.note_type}</Badge><Badge>{note.status}</Badge></div>
              </div>
              <ActionButton onClick={() => convert.mutate(note.id)} disabled={convert.isPending}><Plus className="h-4 w-4" /> Convert</ActionButton>
            </div>
          </div>
        ))}
      </div>
      <ListState loading={notes.isLoading} error={notes.error} empty={!notes.data?.length} />
    </Guard>
  );
}

function CandidateList({
  candidates,
  loading,
  error,
  approve,
  reject,
}: {
  candidates?: AdminKnowledgeCandidate[];
  loading: boolean;
  error: unknown;
  approve: (id: string) => void;
  reject: (id: string) => void;
}) {
  return (
    <>
      <ListState loading={loading} error={error} empty={!candidates?.length} />
      <div className="mt-4 grid gap-3">
        {candidates?.map((candidate) => (
          <div key={candidate.id} className="rounded-md border border-ink/10 bg-white p-4">
            <div className="flex flex-wrap justify-between gap-3">
              <div>
                <Link className="font-semibold text-brand" to={`/admin/knowledge/candidates/${candidate.id}`}>{candidate.title_ru}</Link>
                <p className="mt-1 text-sm text-ink/65">{candidate.summary_ru}</p>
                <div className="mt-2 flex flex-wrap gap-2"><Badge>{candidate.status}</Badge><Badge>{candidate.risk_level}</Badge>{candidate.category_slug && <Badge>{candidate.category_slug}</Badge>}</div>
              </div>
              <div className="flex gap-2">
                <ActionButton onClick={() => approve(candidate.id)}><Check className="h-4 w-4" /> Approve</ActionButton>
                <ActionButton onClick={() => reject(candidate.id)}><X className="h-4 w-4" /> Reject</ActionButton>
              </div>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-ink/10 bg-white p-5">
      <div className="text-sm text-ink/55">{label}</div>
      <div className="mt-2 text-2xl font-semibold">{value}</div>
    </div>
  );
}

function ProfileCard({ profile }: { profile: AdminAssistantProfile }) {
  const status = useMemo(() => (profile.is_active ? "active" : "inactive"), [profile.is_active]);
  return (
    <div className="rounded-md border border-ink/10 bg-white p-4">
      <h2 className="font-semibold">{profile.name}</h2>
      <p className="mt-1 text-sm text-ink/65">{profile.slug}</p>
      <div className="mt-2 flex flex-wrap gap-2"><Badge>{status}</Badge><Badge>prompt {profile.active_prompt_version_id ? "set" : "fallback"}</Badge><Badge>policy {profile.active_policy_version_id ? "set" : "fallback"}</Badge></div>
    </div>
  );
}

export function ListState({ loading, error, empty }: { loading: boolean; error: unknown; empty: boolean }) {
  if (loading) return <div className="mt-4"><LoadingState /></div>;
  if (error) return <div className="mt-4"><ErrorState error={error} /></div>;
  if (empty) return <div className="mt-4"><EmptyState label="No records" /></div>;
  return null;
}
