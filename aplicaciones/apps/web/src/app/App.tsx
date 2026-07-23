import { Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "../components/layout/AppLayout";
import { AboutPage } from "../pages/AboutPage";
import { AdminPage } from "../pages/AdminPage";
import {
  AdminAssistantCandidateDetailPage,
  AdminAssistantCandidatesPage,
  AdminAssistantNotesPage,
  AdminAssistantPage,
  AdminAssistantProfilesPage,
  AdminKnowledgeCandidateDetailPage,
  AdminKnowledgeCandidatesPage,
  AdminKnowledgePage,
  AdminKnowledgeSourcesPage,
} from "../pages/AdminToolsPages";
import { AssistantPage } from "../pages/AssistantPage";
import { DocumentAnalyzerPage } from "../pages/DocumentAnalyzerPage";
import { GuidesPage } from "../pages/GuidesPage";
import { GuideDetailPage } from "../pages/GuideDetailPage";
import { HomePage } from "../pages/HomePage";
import { KbPage } from "../pages/KbPage";
import { LegalPage } from "../pages/LegalPage";
import { LegalReviewPage } from "../pages/LegalReviewPage";
import { PricingPage } from "../pages/PricingPage";
import { SearchPage } from "../pages/SearchPage";
import { ArticlePage } from "../pages/ArticlePage";

export function App() {
  return (
    <AppLayout>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/guides" element={<GuidesPage />} />
        <Route path="/guides/:slug" element={<GuideDetailPage />} />
        <Route path="/kb" element={<KbPage />} />
        <Route path="/kb/:slug" element={<ArticlePage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/assistant" element={<AssistantPage />} />
        <Route path="/document-analyzer" element={<DocumentAnalyzerPage />} />
        <Route path="/pricing" element={<PricingPage />} />
        <Route path="/about" element={<AboutPage />} />
        <Route path="/legal" element={<LegalPage />} />
        <Route path="/admin" element={<AdminPage />} />
        <Route path="/admin/legal" element={<LegalReviewPage />} />
        <Route path="/admin/knowledge" element={<AdminKnowledgePage />} />
        <Route path="/admin/knowledge/sources" element={<AdminKnowledgeSourcesPage />} />
        <Route path="/admin/knowledge/candidates" element={<AdminKnowledgeCandidatesPage />} />
        <Route path="/admin/knowledge/candidates/:id" element={<AdminKnowledgeCandidateDetailPage />} />
        <Route path="/admin/assistant" element={<AdminAssistantPage />} />
        <Route path="/admin/assistant/profiles" element={<AdminAssistantProfilesPage />} />
        <Route path="/admin/assistant/candidates" element={<AdminAssistantCandidatesPage />} />
        <Route path="/admin/assistant/candidates/:id" element={<AdminAssistantCandidateDetailPage />} />
        <Route path="/admin/assistant/notes" element={<AdminAssistantNotesPage />} />
        <Route path="*" element={<Navigate replace to="/" />} />
      </Routes>
    </AppLayout>
  );
}
