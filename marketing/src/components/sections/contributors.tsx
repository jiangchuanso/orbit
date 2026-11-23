import Image from "next/image";
import { GithubIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { ButtonLink } from "@/components/ui";

import { getT } from "@/i18n/t";
import type { Locale } from "@/i18n/config";
import { getContributors } from "@/lib/releases";
import { SITE } from "@/lib/site";

/** Avatars shown in the stack before the "+N" chip takes over. */
const MAX_AVATARS = 32;

const GRAPH_URL = `${SITE.github}/graphs/contributors`;

/**
 * Everyone who has contributed, as the closing panel of the Reviews band: what
 * users say, then who builds it. On its own it was a near-empty band of one
 * centred avatar row; here it shares the reviews' left edge and closes them.
 * Avatars overlap, the most active contributor sits first, and each one links
 * to their profile. Renders nothing if GitHub is unavailable.
 */
export async function ContributorsPanel({ locale }: { locale: Locale }) {
  const t = getT(locale, "sections.contributors");
  const contributors = await getContributors();
  if (contributors.length === 0) return null;

  const shown = contributors.slice(0, MAX_AVATARS);
  const extra = contributors.length - shown.length;

  return (
    <div
      id="contributors"
      className="mt-16 grid scroll-mt-24 grid-cols-1 items-center gap-8 rounded-[18px] border border-edge-default bg-surface-1 p-7 sm:p-9 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-14"
    >
      <div>
        <p className="m-0 font-mono text-[11.5px] uppercase tracking-[0.14em] text-ink-3">
          {t("label")}
        </p>
        <h3 className="mt-3 text-balance text-[clamp(22px,2vw,26px)] font-semibold leading-[1.2] tracking-[-0.02em] text-ink">
          {t("heading")}
        </h3>
        <p className="mt-3 max-w-[46ch] text-[15px] leading-[1.7] text-ink-2">{t("body")}</p>
      </div>

      <div className="flex flex-col items-start gap-6 lg:items-end">
        <ul className="isolate m-0 flex list-none flex-wrap items-center p-0 pl-2 lg:justify-end">
          {shown.map((c) => (
            <li
              key={c.login}
              className="relative -ml-2 transition-transform duration-200 hover:z-10 hover:-translate-y-0.5"
            >
              <a
                href={c.profileUrl}
                target="_blank"
                rel="noreferrer"
                aria-label={c.login}
                className="block rounded-full ring-2 ring-surface-1"
              >
                <Image
                  src={c.avatarUrl}
                  alt=""
                  width={40}
                  height={40}
                  className="size-10 rounded-full border border-edge-default"
                />
              </a>
            </li>
          ))}

          {extra > 0 ? (
            <li className="relative -ml-2">
              <a
                href={GRAPH_URL}
                target="_blank"
                rel="noreferrer"
                aria-label={t("more", { count: extra })}
                className="flex size-10 items-center justify-center rounded-full border border-edge-default bg-surface-2 font-mono text-[11px] text-ink-2 ring-2 ring-surface-1 no-underline transition-colors hover:text-ink"
              >
                +{extra}
              </a>
            </li>
          ) : null}
        </ul>

        <ButtonLink href={GRAPH_URL} variant="secondary" external className="min-h-11 px-5 text-[13px]">
          <HugeiconsIcon icon={GithubIcon} data-icon="inline-start" />
          {t("github")}
        </ButtonLink>
      </div>
    </div>
  );
}
