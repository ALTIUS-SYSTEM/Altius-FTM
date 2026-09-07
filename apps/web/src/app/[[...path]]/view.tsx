"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { useDemo } from "@/components/demo-provider";
import { Shell } from "@/components/shell";
import { DemoLogin } from "@/features/login";
import { Callback } from "@/features/callback";
import { TaskBoard } from "@/features/tasks";
import { Dashboard, Tracking, Schedule, Gallery, RouteVisit, RouteConfig, RouteResult, FlowBuilder, Automation, WorkflowList, DataList, DataType, DataImport, DataExport, Users, Teams, Permissions, Hubs, Organization, CustomModule, Trash, NoAccess, Plan, Subscription, History, Lhs, Anomaly } from "@/features/views";
import { VIEW_PATHS } from "@/lib/routes";

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
  useEffect(() => {
    if (!ready) return;
    if (state.session && path === "") router.replace("/dashboard/task");
    if (!state.session && path !== "" && path !== "login") router.replace("/login");
  }, [ready, state.session, path, router]);
  if (!ready) return <main className="boot"><p>Loading demo workspace…</p></main>;
  if (target === "callback") return <Callback />;
  if (!state.session || target === null) return <DemoLogin/>;
  const Component = VIEWS[target] ?? Dashboard;
  return <Shell path={VIEW_PATHS.includes(target) || target === "lhs" || target === "anomaly" ? target : "dashboard/task"}><Component/></Shell>;
}
