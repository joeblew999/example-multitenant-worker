import { Loader } from "@cloudflare/kumo";

/**
 * Shared loading affordance for in-flight RPC fetches and route guards.
 *
 * Replaces two bad patterns we used to spray everywhere:
 *   - Bare <p className="loading">…</p>  →  looked like a 1999 page mid-load.
 *   - <Empty title="Loading…">           →  semantically wrong; Empty is
 *                                            for "no data here", not "fetching."
 *
 * Centers a Kumo Loader + optional label. Use it for any state where the
 * page can't render its content yet because data hasn't arrived.
 */
export function PageLoading({ label }: { label?: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-12 text-kumo-subtle">
      <Loader size="lg" />
      {label && (
        <span className="font-mono text-sm uppercase tracking-wider">
          {label}
        </span>
      )}
    </div>
  );
}
