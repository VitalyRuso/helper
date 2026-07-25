import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Play, X } from "lucide-react";
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { ErrorState } from "../components/ui/State";
import { ActionButton, Badge, Guard, ListState } from "./AdminToolsPages";

function token() {
  return localStorage.getItem("admin-token") ?? "";
}

export function LegalReviewPage() {
  const t = token();
  const queryClient = useQueryClient();
  const [selectedTaskId, setSelectedTaskId] = useState("");
  const [note, setNote] = useState("");
  const [success, setSuccess] = useState("");

  const tasks = useQuery({
    queryKey: ["admin-legal-review-tasks"],
    queryFn: () => api.adminLegalReviewTasks(t),
    enabled: Boolean(t),
  });
  const details = useQuery({
    queryKey: ["admin-legal-review-task", selectedTaskId],
    queryFn: () => api.adminLegalReviewTask(t, selectedTaskId),
    enabled: Boolean(t && selectedTaskId),
  });
  const knowledge = useQuery({
    queryKey: ["admin-legal-knowledge"],
    queryFn: () => api.adminLegalKnowledge(t),
    enabled: Boolean(t),
  });
  // ponytail: the admin endpoint is unpaginated; paginate when legal knowledge volume warrants it.
  const approvedKnowledge = useMemo(
    () => knowledge.data?.items.filter((item) => item.status === "approved" || item.status === "published") ?? [],
    [knowledge.data],
  );

  const refresh = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ["admin-legal-review-tasks"] }),
      queryClient.invalidateQueries({ queryKey: ["admin-legal-knowledge"] }),
    ]);

  const fixture = useMutation({
    mutationFn: () => api.adminLegalRunFixture(t),
    onMutate: () => setSuccess(""),
    onSuccess: async (data) => {
      setSelectedTaskId(data.result.review_task.id);
      setSuccess("Fixture created.");
      await refresh();
    },
  });
  const decision = useMutation({
    mutationFn: ({ id, action }: { id: string; action: "approve" | "reject" }) =>
      action === "approve"
        ? api.adminLegalApproveTask(t, id, note)
        : api.adminLegalRejectTask(t, id, note),
    onMutate: () => setSuccess(""),
    onSuccess: async (_, variables) => {
      setSuccess(`Task ${variables.action === "approve" ? "approved" : "rejected"}.`);
      setSelectedTaskId("");
      setNote("");
      await refresh();
    },
  });

  const task = details.data?.task;

  return (
    <Guard title="Legal Review">
      <div className="flex flex-wrap items-center gap-3">
        {import.meta.env.DEV && (
          <ActionButton onClick={() => fixture.mutate()} disabled={fixture.isPending}>
            <Play className="h-4 w-4" /> Run fixture
          </ActionButton>
        )}
        {success && <p className="text-sm font-medium text-brand">{success}</p>}
      </div>
      {(fixture.error || decision.error) && (
        <div className="mt-4">
          <ErrorState error={fixture.error ?? decision.error} />
        </div>
      )}

      <div className="mt-6 grid gap-5 lg:grid-cols-[minmax(16rem,0.8fr)_minmax(0,2fr)]">
        <section>
          <h2 className="text-xl font-semibold">Pending tasks</h2>
          <ListState loading={tasks.isLoading} error={tasks.error} empty={!tasks.data?.items.length} />
          <div className="mt-3 space-y-2">
            {tasks.data?.items.map((item) => (
              <button
                key={item.id}
                type="button"
                aria-pressed={selectedTaskId === item.id}
                className={`w-full rounded-md border bg-white p-4 text-left hover:border-brand/40 ${
                  selectedTaskId === item.id ? "border-brand" : "border-ink/10"
                }`}
                onClick={() => {
                  setSelectedTaskId(item.id);
                  setNote("");
                  setSuccess("");
                }}
              >
                <span className="font-semibold">{item.title}</span>
                <span className="mt-2 flex flex-wrap gap-2">
                  <Badge>{item.status}</Badge>
                  <Badge>{item.priority}</Badge>
                </span>
              </button>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold">Task details</h2>
          <ListState loading={details.isLoading} error={details.error} empty={!selectedTaskId} />
          {details.data && task && (
            <article className="mt-3 space-y-4 rounded-md border border-ink/10 bg-white p-5">
              <div>
                <div className="flex flex-wrap gap-2">
                  <Badge>{task.status}</Badge>
                  <Badge>{task.priority}</Badge>
                  <Badge>{details.data.currentness.is_stale ? "stale version" : "current version"}</Badge>
                </div>
                <h3 className="mt-3 text-2xl font-semibold">{task.title}</h3>
                <p className="mt-2 text-sm text-ink/70">{task.ai_summary || "No AI summary."}</p>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <InfoPanel title="Legal change">
                  <p>{details.data.legal_change.detected_summary}</p>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <Badge>{details.data.legal_change.change_type}</Badge>
                    <Badge>{details.data.legal_change.status}</Badge>
                  </div>
                </InfoPanel>
                <InfoPanel title="Document">
                  <p className="font-medium">{details.data.document.title}</p>
                  <p className="mt-1">{details.data.document.official_id ?? details.data.document.eli_id ?? "No official ID"}</p>
                  <p className="mt-1">{details.data.document.legal_area} / {details.data.document.procedure_key ?? "no procedure"}</p>
                </InfoPanel>
              </div>

              <InfoPanel title="Diff context">
                <p>{details.data.diff.summary || "No diff summary."}</p>
                <details className="mt-3">
                  <summary className="cursor-pointer font-medium text-brand">Raw diff</summary>
                  <pre className="mt-2 max-h-72 overflow-auto whitespace-pre-wrap rounded-md bg-paper p-3 text-xs">
                    {JSON.stringify(details.data.diff.diff_json, null, 2)}
                  </pre>
                </details>
              </InfoPanel>

              <div>
                <h4 className="font-semibold">Affected sections</h4>
                <div className="mt-3 space-y-3">
                  {details.data.affected_sections.map((section) => (
                    <div key={section.id} className="rounded-md bg-paper p-4">
                      <p className="font-medium">{section.title || section.stable_section_key}</p>
                      <p className="mt-2 whitespace-pre-wrap text-sm text-ink/75">{section.text_content}</p>
                    </div>
                  ))}
                  {!details.data.affected_sections.length && <p className="text-sm text-ink/55">No affected section text.</p>}
                </div>
              </div>

              <label className="block text-sm font-medium">
                Review note
                <textarea
                  className="mt-2 min-h-28 w-full rounded-md border border-ink/15 px-3 py-2 font-normal"
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  placeholder="Reviewer note"
                />
              </label>
              <p className="text-xs text-ink/55">Reviewer: dev</p>
              <div className="flex flex-wrap gap-2">
                <ActionButton onClick={() => decision.mutate({ id: task.id, action: "approve" })} disabled={decision.isPending}>
                  <Check className="h-4 w-4" /> Approve
                </ActionButton>
                <ActionButton onClick={() => decision.mutate({ id: task.id, action: "reject" })} disabled={decision.isPending}>
                  <X className="h-4 w-4" /> Reject
                </ActionButton>
              </div>
            </article>
          )}
        </section>
      </div>

      <section className="mt-8">
        <h2 className="text-xl font-semibold">Approved Knowledge</h2>
        <ListState loading={knowledge.isLoading} error={knowledge.error} empty={!approvedKnowledge.length} />
        <div className="mt-3 grid gap-3 md:grid-cols-2">
          {approvedKnowledge.map((item) => (
            <article key={item.id} className="rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap gap-2">
                <Badge>{item.status}</Badge>
                <Badge>{item.is_stale ? "stale" : "current"}</Badge>
                <Badge>{item.procedure_key}</Badge>
              </div>
              <h3 className="mt-3 font-semibold">{item.title_es}</h3>
              <p className="mt-2 max-h-24 overflow-hidden whitespace-pre-wrap text-sm text-ink/70">{item.canonical_answer_es}</p>
              <p className="mt-3 text-xs text-ink/50">Approved by {item.approved_by ?? "Legal Reviewer"}</p>
            </article>
          ))}
        </div>
      </section>
    </Guard>
  );
}

function InfoPanel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-md bg-paper p-4 text-sm text-ink/70">
      <h4 className="mb-2 font-semibold text-ink">{title}</h4>
      {children}
    </div>
  );
}
