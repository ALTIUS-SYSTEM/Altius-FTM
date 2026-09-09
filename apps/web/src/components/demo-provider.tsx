"use client";

import { createContext, useContext, useEffect, useState, useCallback, useMemo } from "react";
import type { ReactNode } from "react";
import type { DemoState } from "@/data/model";
import { createEmptyState } from "@/data/model";
import { apiBase, createApiAdapter } from "@/data/api-adapter";
import type { DemoAdapter } from "@/data/adapter";
import { accessToken, authConfig } from "@/lib/auth";
import { translate } from "@/lib/i18n";
import { ZodError } from "zod";

/**
 * The dashboard only runs in live API mode. Missing configuration is reported,
 * never thrown: this runs during render, so a throw here takes out SSR too and
 * turns a setup mistake into a blank 500 with the reason only in server logs.
 */
const pickAdapter = (): { adapter: DemoAdapter | null; configError: string } => {
  const cfg = authConfig();
  const base = apiBase();
  if (!base)
    return { adapter: null, configError: "NEXT_PUBLIC_API_BASE is not set. Point it at the Altius API (e.g. http://127.0.0.1:8080) in apps/web/.env.local." };
  if (!cfg)
    return { adapter: null, configError: "NEXT_PUBLIC_KEYCLOAK_URL, _REALM and _CLIENT_ID must all be set in apps/web/.env.local." };
  return { adapter: createApiAdapter(base, () => accessToken(cfg)), configError: "" };
};

/**
 * A save failure the reader can act on.
 *
 * Zod throws with a JSON array of issues, and `err.message` is that array
 * serialized — so an unmapped field surfaced on screen as a wall of
 * `{"expected":"'Admin' | 'Supervisor'...","code":"invalid_type"}`. Name the
 * fields instead; the detail still goes to the console for whoever is debugging.
 */
function describeSaveFailure(err: unknown): string {
  if (err instanceof ZodError) {
    console.error("workspace state failed validation", err.issues);
    const fields = [...new Set(err.issues.map(i => i.path.join(".") || "workspace"))];
    return `This change could not be saved: ${fields.join(", ")} ${fields.length > 1 ? "are" : "is"} invalid. Reload the page; if it persists, reset the workspace.`;
  }
  return err instanceof Error ? err.message : "This change could not be saved.";
}

type DemoContextValue = { state: DemoState; update: (change: (state: DemoState) => DemoState, message?: string) => void; ready: boolean; error: string; reset: () => void; reload: () => Promise<void>; notify: (message: string) => void; t: (key: string) => string };
const DemoContext = createContext<DemoContextValue | null>(null);
export function DemoProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState(createEmptyState);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  // The adapter is stateless but not free to rebuild, and `save` must not run
  // inside a setState updater: React may invoke an updater twice (StrictMode)
  // or discard its result when a higher-priority update rebases, which would
  // persist a state the UI never committed.
  const { adapter, configError } = useMemo(() => pickAdapter(), []);
  // Fetch the workspace from the API. Exposed because tokens live in memory
  // only: a reload starts signed out, this runs and fails, and the session is
  // then restored silently *afterwards*. Without a way to ask again, the tab
  // kept the empty state it built before the token existed — the account was
  // signed in, the shell showed the user's name, and every record already in
  // the database was invisible until they signed in from scratch.
  const reload = useCallback(async () => {
    if (!adapter) { setError(configError); setReady(true); return; }
    try {
      const data = await adapter.load();
      setState(current => ({
        ...data,
        // A sign-in completing right now is newer than whatever the last save
        // wrote, so it wins; on a plain mount there is no such sign-in and the
        // stored values stand.
        session: current.session || data.session,
        role: current.session ? current.role : data.role,
      }));
      setError("");
    } catch (err: unknown) {
      // Having no token before sign-in is the expected pre-login state, not a
      // failure — surfacing it as an error banner on the sign-in screen tells
      // the user something is broken when nothing is.
      const message = err instanceof Error ? err.message : "";
      setError(message === "not authenticated" ? "" : (message || "The workspace could not load from the API."));
    } finally {
      setReady(true);
    }
  }, [adapter, configError]);
  useEffect(() => { void reload(); }, [reload]);
  useEffect(() => { if (notice) { const timer = setTimeout(() => setNotice(""), 5000); return () => clearTimeout(timer); } }, [notice]);
  useEffect(() => { document.documentElement.lang = state.locale; }, [state.locale]);
  const update = useCallback((change: (current: DemoState) => DemoState, message?: string) => {
    setState(current => {
      const next = change(current);
      // Persist outside the render phase, and only announce success if it
      // actually persisted — the toast used to fire unconditionally.
      queueMicrotask(() => {
        void (async () => {
          try {
            if (!adapter) throw new Error(configError);
            await adapter.save(next);
            setError("");
            if (message) setNotice(message);
          } catch (err) {
            setError(describeSaveFailure(err));
          }
        })();
      });
      return next;
    });
  }, [adapter, configError]);
  const reset = () => {
    if (!adapter) { setError(configError); return; }
    try { setState(adapter.reset()); setError(""); setNotice("Workspace reset."); } catch { setError("Browser storage is unavailable. Check storage permissions."); }
  };
  return <DemoContext.Provider value={{ state, update, ready, error, reset, reload, notify: setNotice, t: key => translate(state.locale, key) }}>{children}<div aria-live="polite" role="status" className={notice ? "toast visible" : "toast"}>{notice}</div></DemoContext.Provider>;
}
export function useDemo() {
  const context = useContext(DemoContext);
  if (!context) throw new Error("DemoProvider is required");
  return context;
}
