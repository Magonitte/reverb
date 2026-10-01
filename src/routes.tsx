import { lazy } from "react";
import { Navigate, Route, Routes } from "react-router";
import { BareLayout, ShellLayout } from "@/components/layout/ShellLayout";

const Home = lazy(() => import("@/screens/Home"));
const Library = lazy(() => import("@/screens/Library"));
const Playlists = lazy(() => import("@/screens/Playlists"));
const Activity = lazy(() => import("@/screens/Activity"));
const Review = lazy(() => import("@/screens/Review"));
const TagEditor = lazy(() => import("@/screens/TagEditor"));
const Settings = lazy(() => import("@/screens/Settings"));
const Onboarding = lazy(() => import("@/screens/Onboarding"));

/** Rotas do design §3. O roteador (HashRouter no app, MemoryRouter nos testes) vem de fora. */
export function AppRoutes() {
  return (
    <Routes>
      <Route element={<ShellLayout />}>
        <Route index element={<Home />} />
        <Route path="library" element={<Library />} />
        <Route path="playlists" element={<Playlists />} />
        <Route path="playlists/:id" element={<Playlists />} />
        <Route path="activity" element={<Activity />} />
        <Route path="review" element={<Review />} />
        <Route path="tag-editor" element={<TagEditor />} />
        <Route path="settings/:tab?" element={<Settings />} />
      </Route>
      <Route element={<BareLayout />}>
        <Route path="onboarding" element={<Onboarding />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
