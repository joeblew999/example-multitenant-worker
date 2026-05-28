import { Text } from "@cloudflare/kumo";
import type { ReactNode } from "react";

/**
 * Shared hero for the three auth-flow pages (Login, Signup, AcceptInvite).
 *
 * Pattern: small mono eyebrow → Doto display title → secondary lede.
 *
 * The <h1 className="font-heading"> gets Doto via theme-editorial-extras.css
 * (which selector-overrides `[data-theme="editorial"] h1.font-heading`
 * to use --font-display). Under the kumo / fedramp themes it falls back
 * to Kumo's default heading font automatically.
 *
 * Note: Kumo's <Text> omits `className` from its prop type when `variant`
 * is set, so the eyebrow uses a plain <p> with kumo-token utility classes
 * for the custom mono/uppercase styling.
 */
export function AuthHero({
  eyebrow,
  title,
  lede,
}: {
  eyebrow: ReactNode;
  title: ReactNode;
  lede?: ReactNode;
}) {
  return (
    <header className="flex flex-col gap-3">
      <p className="font-mono uppercase tracking-wider text-xs text-kumo-subtle m-0">
        {eyebrow}
      </p>
      <h1 className="font-heading text-4xl sm:text-5xl leading-none m-0">
        {title}
      </h1>
      {lede && <Text variant="secondary">{lede}</Text>}
    </header>
  );
}
