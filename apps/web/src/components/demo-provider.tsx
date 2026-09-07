"use client";

import { createContext, useContext, useEffect, useState, useCallback } from "react";
import type { ReactNode } from "react";
import type { DemoState } from "@/data/model";
import { createFixtures } from "@/data/fixtures";
import { createLocalAdapter } from "@/data/adapter";
import { apiBase, createApiAdapter } from "@/data/api-adapter";
import { accessToken, authConfig } from "@/lib/auth";
import { translate } from "@/lib/i18n";

/** Pick the live API adapter when both the API base and Keycloak are configured. */
const pickAdapter = () => {
  const cfg = authConfig();
  if (apiBase() && cfg) return createApiAdapter(apiBase(), () => accessToken(cfg));
  return createLocalAdapter(window.localStorage);
};

type DemoContextValue = { state: DemoState; update: (change: (state: DemoState) => DemoState, message?: string) => void; ready: boolean; error: string; reset: () => void; notify: (message: string) => void; t: (key: string) => string };
const DemoContext = createContext<DemoContextValue | null>(null);
export function DemoProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState(createFixtures);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let active = true;
    pickAdapter().load().then(data => { if (active) { setState(data); setReady(true); } }).catch(() => { if (active) { setError("The browser could not load your demo data. Reset the demo to continue. No server data is affected."); setReady(true); } });
    return () => { active = false; };
  }, []);
  useEffect(() => { if (notice) { const timer = setTimeout(() => setNotice(""), 5000); return () => clearTimeout(timer); } }, [notice]);
  useEffect(() => { document.documentElement.lang = state.locale; }, [state.locale]);
  const update = useCallback((change: (current: DemoState) => DemoState, message?: string) => {
    setState(current => {
      const next = change(current);
      try { pickAdapter().save(next); } catch { setError("Your browser could not save this change. Changes are held in memory only; check storage permissions or reset the demo."); }
      return next;
    });
    if (message) setNotice(message);
  }, []);
  const reset = () => {
    try { setState(pickAdapter().reset()); setError(""); setNotice("Synthetic demo data restored."); } catch { setError("Browser storage is unavailable. Allow local storage and try again."); }
  };
  return <DemoContext.Provider value={{ state, update, ready, error, reset, notify: setNotice, t: key => translate(state.locale, key) }}>{children}<div aria-live="polite" role="status" className={notice ? "toast visible" : "toast"}>{notice}</div></DemoContext.Provider>;
}
export function useDemo() {
  const context = useContext(DemoContext);
  if (!context) throw new Error("DemoProvider is required");
  return context;
}
