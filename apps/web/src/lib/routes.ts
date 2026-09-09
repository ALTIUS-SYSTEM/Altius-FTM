export const MODULES = [
  { key: "dashboard", home: "dashboard/task", tabs: ["task"] },
  { key: "tasks", home: "tasks/task", tabs: ["task", "tracking", "schedule", "gallery"] },
  { key: "route", home: "route/visit", tabs: ["visit", "configuration", "result"] },
  { key: "lhs", home: "lhs", tabs: [] },
  { key: "anomaly", home: "anomaly", tabs: [] },
  { key: "flow", home: "flow/flow", tabs: ["flow", "automation", "workflow"] },
  { key: "data", home: "data/data-list", tabs: ["data-list", "data-type"] },
  { key: "import-export", home: "import-export/data-import", tabs: ["data-import", "data-export"] },
  { key: "setting", home: "setting/user", tabs: ["user", "team", "permission", "hub", "organization", "custom-module", "no-access", "trash"] },
  { key: "billing", home: "billing/plan", tabs: ["plan", "subscription", "history"] }
];
export const VIEW_PATHS = MODULES.flatMap(module => module.tabs.length ? module.tabs.map(tab => tab === "no-access" ? tab : `${module.key}/${tab}`) : [module.home]);
export const PAGE_DESCRIPTIONS: Record<string, string> = {
  dashboard: "A clear view of your field operations. Every task, every team, one workspace.",
  tasks: "Plan the day, assign your team, and keep every delivery moving.",
  route: "Build efficient journeys and compare planned visits with recorded GPS trails.",
  lhs: "Laporan Harian Sopir · Review daily activity, visit evidence, and operational costs.",
  anomaly: "Compare app and vehicle GPS samples. Investigate signals, not assumptions.",
  flow: "Design the steps that turn field activity into consistent, reliable outcomes.",
  data: "Keep the operational data behind your field teams organized and ready.",
  "import-export": "Move synthetic data in and out of this browser. Nothing is uploaded.",
  setting: "Manage your organization, people, and operating preferences.",
  billing: "Explore plans and sample invoices. No payments or subscriptions are created."
};
