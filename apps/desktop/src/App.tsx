import { Routes, Route, Navigate } from "react-router-dom";
import Shell from "./components/Shell";
import Tasks from "./pages/Tasks";
import Repos from "./pages/dashboard/Repos";
import RepositoryDetail from "./pages/repos/RepositoryDetail";
import {
  IssueDetailView,
  PullRequestDetailView,
} from "./pages/detail/DetailView";
import Issues from "./pages/dashboard/Issues";
import Pulls from "./pages/dashboard/Pulls";
import TriageWorkspace from "./pages/triage/TriageWorkspace";
import InboxView from "./pages/inbox/InboxView";
import CIHealthView from "./pages/ci/CIHealthView";
import InsightsView from "./pages/insights/InsightsView";
import DailyDigestView from "./pages/digest/DailyDigestView";
import BoardsList from "./pages/boards/BoardsList";
import BoardView from "./pages/boards/BoardView";
import Settings from "./pages/Settings";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { AccountProvider } from "./contexts/AccountContext";
import { CommandPaletteProvider } from "./contexts/CommandPaletteContext";
import CommandPalette from "./components/command/CommandPalette";
import { queryClient, persister, ONE_DAY } from "./lib/query/client";

export default function App() {
  return (
    <PersistQueryClientProvider
      client={queryClient}
      persistOptions={{ persister, maxAge: ONE_DAY }}
    >
      <AccountProvider>
      <CommandPaletteProvider>
        <Shell>
          <Routes>
            <Route path="/" element={<Tasks />} />
            <Route path="/repos" element={<Repos />} />
            <Route
              path="/repos/:owner/:repo"
              element={<RepositoryDetail />}
            />
            <Route
              path="/repos/:owner/:repo/issues/:number"
              element={<IssueDetailView />}
            />
            <Route
              path="/repos/:owner/:repo/pull/:number"
              element={<PullRequestDetailView />}
            />
            <Route path="/issues" element={<Issues />} />
            <Route path="/pulls" element={<Pulls />} />
            <Route path="/triage" element={<TriageWorkspace />} />
            <Route path="/inbox" element={<InboxView />} />
            <Route path="/ci" element={<CIHealthView />} />
            <Route path="/insights" element={<InsightsView />} />
            <Route path="/digest" element={<DailyDigestView />} />
            <Route path="/boards" element={<BoardsList />} />
            <Route path="/boards/:boardId" element={<BoardView />} />
            <Route
              path="/dashboard"
              element={<Navigate to="/repos" replace />}
            />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </Shell>
        <CommandPalette />
      </CommandPaletteProvider>
      </AccountProvider>
    </PersistQueryClientProvider>
  );
}
