import type { ReactNode } from "react";

type ProseProps = {
  children: ReactNode;
};

/**
 * Typography wrapper for MDX article bodies.
 */
export function Prose({ children }: ProseProps) {
  return <div className="prose-docs">{children}</div>;
}
