"use client";

import { createContext, useContext, useEffect, useState, useCallback, useMemo } from "react";
import type { ReactNode } from "react";
import type { DemoState } from "@/data/model";
import { createEmptyState } from "@/data/model";
import { apiBase, createApiAdapter } from "@/data/api-adapter";
import { accessToken, authConfig } from "@/lib/auth";
import { translate } from "@/lib/i18n";

/** The dashboard only runs in live API mode. */
const pickAdapter = () => {
  const cfg = authConfig();
  const base = apiBase();
  if (!base) throw new Error("NEXT_PUBLIC_API_BASE is required");
  if (!cfg) throw new Error("NEXT_PUBLIC_KEYCLOAK_* configuration is required");
  return createApiAdapter(base, () => accessToken(cfg));
};

type DemoContextValue = { state: DemoState; update: (change: (state: DemoState) => DemoState, message?: string) => void; ready: boolean; error: string; reset: () => void; notify: (message: string) => void; t: (key: string) => string };
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
  const adapter = useMemo(() => pickAdapter(), []);
  useEffect(() => {
    let active = true;
    adapter.load().then(data => { if (active) { setState(data); setReady(true); } }).catch((err) => { if (active) { setError(err instanceof Error ? err.message : "The workspace could not load from the API."); setReady(true); } });
    return () => { active = false; };
  }, [adapter]);
  useEffect(() => { if (notice) { const timer = setTimeout(() => setNotice(""), 5000); return () => clearTimeout(timer); } }, [notice]);
  useEffect(() => { document.documentElement.lang = state.locale; }, [state.locale]);
  const update = useCallback((change: (current: DemoState) => DemoState, message?: string) => {
    setState(current => {
      const next = change(current);
      // Persist outside the render phase, and only announce success if it
      // actually persisted — the toast used to fire unconditionally.
      queueMicrotask(() => {
        try {
          adapter.save(next);
          setError("");
          if (message) setNotice(message);
        } catch (err) {
          setError(err instanceof Error ? err.message : "This change could not be saved.");
        }
      });
      return next;
    });
  }, [adapter]);
  const reset = () => {
    try { setState(adapter.reset()); setError(""); setNotice("Workspace reset."); } catch { setError("Browser storage is unavailable. Check storage permissions."); }
  };
  return <DemoContext.Provider value={{ state, update, ready, error, reset, notify: setNotice, t: key => translate(state.locale, key) }}>{children}<div aria-live="polite" role="status" className={notice ? "toast visible" : "toast"}>{notice}</div></DemoContext.Provider>;
}
export function useDemo() {
  const context = useContext(DemoContext);
  if (!context) throw new Error("DemoProvider is required");
  return context;
}
