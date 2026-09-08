"use client";

import { useEffect, useId, useRef } from "react";
import type { ReactNode } from "react";
import { useDemo } from "./demo-provider";

export function Brand({ compact = false }: { compact?: boolean }) {
  return <span className="brand"><img src="/altius-logo.svg" alt="" width="38" height="38" className="brand-mark"/>{!compact && <span>altius<span className="brand-dot">.</span><small>FIELD OPERATIONS</small></span>}</span>;
}
const iconPaths: Record<string, string> = { dashboard: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z", tasks: "M8 5H5v16h14V5h-3 M9 3h6v4H9z M8 12l2 2 5-5 M8 18h7", route: "M5 4a2 2 0 1 0 0 .1 M19 18a2 2 0 1 0 0 .1 M5 7v8a4 4 0 0 0 4 4h3 M19 15V7a3 3 0 0 0-3-3h-4", flow: "M9 2h6v6H9z M2 16h6v6H2z M16 16h6v6h-6z M12 8v4H5v4 M12 12h7v4", data: "M3 5c0-4 18-4 18 0s-18 4-18 0v14c0 4 18 4 18 0V5 M3 12c0 4 18 4 18 0", "import-export": "M7 3v15 M3 14l4 4 4-4 M17 21V6 M13 10l4-4 4 4", setting: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M10 2h4l1 3 3 1 3 3-1 3 1 3-3 3-3 1-1 3h-4l-1-3-3-1-3-3 1-3-1-3 3-3 3-1z", billing: "M3 5h18v14H3z M3 10h18 M6 15h4", lhs: "M5 3h14v18H5z M8 7h8 M8 11h8 M8 15h4", anomaly: "M12 3 2 21h20z M12 9v5 M12 17v1", menu: "M4 6h16 M4 12h16 M4 18h16", search: "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14 M15 15l6 6", bell: "M6 8a6 6 0 0 1 12 0v7l2 3H4l2-3z M10 21h4", plus: "M12 5v14 M5 12h14", arrow: "M5 12h14 M14 7l5 5-5 5", check: "M5 12l4 4L19 6", pin: "M12 22s8-8 8-14a8 8 0 0 0-16 0c0 6 8 14 8 14z M12 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6", close: "M6 6l12 12 M18 6 6 18", download: "M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5", lock: "M5 11h14v10H5z M8 11V7a4 4 0 0 1 8 0v4 M12 15v2" };
export function Icon({ name, size = 20 }: { name: string; size?: number }) { return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d={iconPaths[name] ?? iconPaths.tasks}/></svg>; }
export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: string }) { return <span className={`badge ${tone}`}><span className="status-dot"/>{children}</span>; }
export function StatusBadge({ status }: { status: string }) { const { t } = useDemo(); return <Badge tone={status}>{t(status)}</Badge>; }
export function Card({ title, action, children, className = "" }: { title?: string; action?: ReactNode; children: ReactNode; className?: string }) { return <section className={`card ${className}`}>{title && <div className="card-head"><h2>{title}</h2>{action}</div>}{children}</section>; }
export function Field({ label, children, hint }: { label: string; children: ReactNode; hint?: string }) { return <label className="field"><span>{label}</span>{children}{hint && <small>{hint}</small>}</label>; }
export function Empty({ title = "No matching records", description = "Try changing your filters or create your first record.", action }: { title?: string; description?: string; action?: ReactNode }) { return <div className="empty"><div className="empty-symbol"><Icon name="data" size={32}/></div><h3>{title}</h3><p>{description}</p>{action}</div>; }
export function Modal({ title, children, onClose, wide = false }: { title: string; children: ReactNode; onClose: () => void; wide?: boolean }) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const id = useId();
  useEffect(() => {
    const dialog = dialogRef.current;
    const previous = document.activeElement as HTMLElement | null;
    dialog?.showModal();
    const overflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => { dialog?.close(); document.body.style.overflow = overflow; previous?.focus(); };
  }, []);
  return <dialog ref={dialogRef} aria-labelledby={id} className={wide ? "modal wide" : "modal"} onCancel={event => { event.preventDefault(); onClose(); }}><div className="modal-head"><div><span className="eyebrow">LOCAL DEMO</span><h2 id={id}>{title}</h2></div><button className="icon-button" aria-label="Close dialog" onClick={onClose}><Icon name="close"/></button></div><div className="modal-body">{children}</div></dialog>;
}
export function Table({ headings, children, count }: { headings: ReactNode[]; children: ReactNode; count?: number }) { return <div className="table-wrap"><table><thead><tr>{headings.map((heading, i) => <th key={i} scope="col">{heading}</th>)}</tr></thead><tbody>{children}</tbody></table>{count !== undefined && <div className="table-foot">{count} record{count === 1 ? "" : "s"} · Synthetic demo data</div>}</div>; }
export function Metric({ label, value, detail, tone }: { label: string; value: ReactNode; detail: string; tone?: string }) { return <div className={`metric ${tone ?? ""}`}><span className="eyebrow">{label}</span><strong>{value}</strong><small>{detail}</small></div>; }
