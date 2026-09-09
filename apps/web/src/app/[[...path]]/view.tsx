"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useDemo } from "@/components/demo-provider";
import { Shell } from "@/components/shell";
import { DemoLogin } from "@/features/login";
import { Callback } from "@/features/callback";
import { TaskBoard } from "@/features/tasks";
import { Dashboard, Tracking, Schedule, Gallery, RouteVisit, RouteConfig, RouteResult, FlowBuilder, Automation, WorkflowList, DataList, DataType, DataImport, DataExport, Users, Teams, Permissions, Hubs, Organization, CustomModule, Trash, NoAccess, Plan, Subscription, History, Lhs, Anomaly } from "@/features/views";
import { VIEW_PATHS } from "@/lib/routes";
import { authConfig, hasSession, silentAuth, startLogin } from "@/lib/auth";

const VIEWS: Record<string, () => React.ReactNode> = {
  callback: Callback,
  "dashboard/task": Dashboard, "tasks/task": TaskBoard, "tasks/tracking": Tracking, "tasks/schedule": Schedule, "tasks/gallery": Gallery,
  "route/visit": RouteVisit, "route/configuration": RouteConfig, "route/result": RouteResult,
  "flow/flow": FlowBuilder, "flow/automation": Automation, "flow/workflow": WorkflowList,
  "data/data-list": DataList, "data/data-type": DataType,
  "import-export/data-import": DataImport, "import-export/data-export": DataExport,
  "setting/user": Users, "setting/team": Teams, "setting/permission": Permissions, "setting/hub": Hubs,
  "setting/organization": Organization, "setting/custom-module": CustomModule, "setting/trash": Trash,
  "billing/plan": Plan, "billing/subscription": Subscription, "billing/history": History,
  lhs: Lhs, anomaly: Anomaly, "no-access": NoAccess,
};

export function View({ path }: { path: string }) {
  const { state, ready } = useDemo();
  const router = useRouter();
  const target = path === "" || path === "login" ? null : path;
  // When Keycloak is configured, authentication is decided by holding a token,
  // not by a persisted `session` flag: the flag can be set by the demo login,
  // written straight into storage, or left true after a failed refresh already
  // cleared the credentials.
  const authenticated = authConfig() ? hasSession() : state.session;
  // Tokens are memory-only, so every reload starts signed out even though the
  // IdP still considers the user logged in. Ask Keycloak once, silently, before
  // sending anyone back to a login form they do not need.
  const [restoring, setRestoring] = useState(false);
  useEffect(() => {
    if (!ready || authenticated || path === "callback") return;
    const cfg = authConfig();
    if (!cfg || silentAuth.tried()) return;
    silentAuth.markTried();
    if (path !== "" && path !== "login") silentAuth.rememberReturn(`/${path}`);
    setRestoring(true);
    void startLogin(cfg, `${window.location.origin}/callback`, { prompt: "none" })
      .catch(() => setRestoring(false));
  }, [ready, authenticated, path]);
  useEffect(() => {
    if (!ready || restoring) return;
    if (authenticated && path === "") router.replace("/dashboard/task");
    if (!authenticated && path !== "" && path !== "login" && path !== "callback") router.replace("/login");
  }, [ready, authenticated, path, router, restoring]);
  if (!ready) return <main className="boot"><p>Loading workspace…</p></main>;
  if (restoring) return <main className="boot"><p>Restoring your session…</p></main>;
  if (target === "callback") return <Callback />;
  if (!authenticated || target === null) return <DemoLogin/>;
  const Component = VIEWS[target] ?? Dashboard;
  return <Shell path={VIEW_PATHS.includes(target) || target === "lhs" || target === "anomaly" ? target : "dashboard/task"}><Component/></Shell>;
}
