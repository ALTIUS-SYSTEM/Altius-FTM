import type { ComponentPropsWithoutRef, ReactNode } from "react";

type CalloutProps = {
  children?: ReactNode;
};

/**
 * Tip callout for MDX guides.
 */
export function Tip({ children }: CalloutProps) {
  return (
    <aside className="my-6 rounded-lg border border-primary-container/40 bg-secondary-container/40 px-4 py-3 text-sm text-on-secondary-container">
      <p className="mb-1 text-label-bold uppercase tracking-wide text-primary">Tip</p>
      <div className="[&>p]:mb-0">{children}</div>
    </aside>
  );
}

/**
 * Warning callout for MDX guides.
 */
export function Warning({ children }: CalloutProps) {
  return (
    <aside className="my-6 rounded-lg border border-tertiary-container/50 bg-tertiary-fixed/50 px-4 py-3 text-sm text-on-tertiary-fixed-variant">
      <p className="mb-1 text-label-bold uppercase tracking-wide text-tertiary">Warning</p>
      <div className="[&>p]:mb-0">{children}</div>
    </aside>
  );
}

type HeadingProps = ComponentPropsWithoutRef<"h1">;
type AnchorProps = ComponentPropsWithoutRef<"a">;
type CodeProps = ComponentPropsWithoutRef<"code">;

/**
 * MDX element map for guide rendering.
 */
export const mdxComponents = {
  Tip,
  Warning,
  h1: (props: HeadingProps) => <h1 className="mb-4 text-headline-lg text-on-surface" {...props} />,
  h2: (props: HeadingProps) => (
    <h2 className="mb-3 mt-10 scroll-mt-24 text-headline-md text-on-surface" {...props} />
  ),
  h3: (props: HeadingProps) => (
    <h3 className="mb-2 mt-8 scroll-mt-24 text-headline-sm text-on-surface" {...props} />
  ),
  p: (props: ComponentPropsWithoutRef<"p">) => (
    <p className="mb-4 leading-relaxed text-on-surface-variant" {...props} />
  ),
  ul: (props: ComponentPropsWithoutRef<"ul">) => (
    <ul className="mb-4 ml-5 list-disc space-y-2 text-on-surface-variant" {...props} />
  ),
  ol: (props: ComponentPropsWithoutRef<"ol">) => (
    <ol className="mb-4 ml-5 list-decimal space-y-2 text-on-surface-variant" {...props} />
  ),
  a: (props: AnchorProps) => (
    <a className="text-primary underline-offset-2 hover:underline" {...props} />
  ),
  code: (props: CodeProps) => {
    const isBlock = typeof props.className === "string" && props.className.includes("language-");
    if (isBlock) {
      return <code className="font-mono text-mono-data" {...props} />;
    }
    return (
      <code
        className="rounded bg-surface-container px-1.5 py-0.5 font-mono text-mono-data text-on-surface"
        {...props}
      />
    );
  },
  pre: (props: ComponentPropsWithoutRef<"pre">) => (
    <pre
      className="mb-4 overflow-x-auto rounded-lg border border-outline-variant bg-surface-container-low p-4"
      {...props}
    />
  ),
  table: (props: ComponentPropsWithoutRef<"table">) => (
    <div className="mb-6 overflow-x-auto">
      <table className="w-full border-collapse text-body-md" {...props} />
    </div>
  ),
  th: (props: ComponentPropsWithoutRef<"th">) => (
    <th
      className="border border-outline-variant bg-surface-container-low px-3 py-2 text-left font-semibold text-on-surface"
      {...props}
    />
  ),
  td: (props: ComponentPropsWithoutRef<"td">) => (
    <td
      className="border border-outline-variant px-3 py-2 text-left text-on-surface-variant"
      {...props}
    />
  ),
  blockquote: (props: ComponentPropsWithoutRef<"blockquote">) => (
    <blockquote className="mb-4 border-l-4 border-primary pl-4 text-on-surface-variant" {...props} />
  ),
  hr: () => <hr className="my-8 border-outline-variant" />,
};
