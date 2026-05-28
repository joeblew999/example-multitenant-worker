import type { ReactNode } from "react";

/**
 * Definition list rendered as a two-column grid: uppercase mono key on
 * the left, free-form value on the right, with a thin divider between
 * rows.
 *
 * Originally lived inline in Dashboard.tsx; extracted because
 * AcceptInvite needs the same shape for the invitation Details block.
 *
 * `dtWidth` lets narrow card layouts (480px auth cards) shrink the key
 * column without forcing it via Tailwind class permutations. Defaults
 * to the wider Dashboard sizing.
 */
export type KvRow = { k: string; v: ReactNode };

export function KvList({
  rows,
  dtWidth = "minmax(140px, 200px)",
}: {
  rows: KvRow[];
  dtWidth?: string;
}) {
  return (
    <dl
      className="grid gap-x-6"
      style={{ gridTemplateColumns: `${dtWidth} 1fr` }}
    >
      {rows.map(({ k, v }, i) => (
        <div key={k} className="contents">
          <dt
            className={`py-3 text-sm uppercase tracking-wider text-kumo-subtle ${
              i > 0 ? "border-t border-kumo-line" : ""
            }`}
          >
            {k}
          </dt>
          <dd
            className={`py-3 text-kumo-default ${
              i > 0 ? "border-t border-kumo-line" : ""
            }`}
          >
            {v}
          </dd>
        </div>
      ))}
    </dl>
  );
}
