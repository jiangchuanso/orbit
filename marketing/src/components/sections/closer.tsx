import Link from "next/link";
import { Download01Icon, GithubIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { OrbitIcon } from "@/components/brand";
import { OsDownloadButton } from "@/components/download-button";
import { Band, ButtonLink, Shell } from "@/components/ui";
import { getLatestRelease, LATEST_RELEASE_URL } from "@/lib/releases";
import { NO_DOWNLOADS, preferredDownload } from "@/lib/downloads";
import { getT } from "@/i18n/t";
import { downloadLabels } from "@/i18n/labels";
import type { Locale } from "@/i18n/config";

export async function Closer({ locale }: { locale: Locale }) {
  const t = getT(locale, "closer");
  const release = await getLatestRelease();
  const downloads = release?.downloads ?? NO_DOWNLOADS;
  const platforms = [
    {
      label: t("platforms.macos"),
      href: preferredDownload(downloads, "macos")?.url ?? LATEST_RELEASE_URL,
    },
    {
      label: t("platforms.windows"),
      href: preferredDownload(downloads, "windows")?.url ?? LATEST_RELEASE_URL,
    },
    {
      label: t("platforms.linux"),
      href: preferredDownload(downloads, "linux")?.url ?? LATEST_RELEASE_URL,
    },
  ];

  return (
    <Shell>
      <Band id="install" className="scroll-mt-20 py-24 sm:py-32">
        {/* The close shares the page's left line: the ask on the left, the
         * downloads on the right column's start, both ending on one baseline. */}
        <div className="grid grid-cols-1 items-end gap-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-14">
          <div>
            <div className="relative mb-8">
              <OrbitIcon
                alt=""
                className="relative h-12 w-12 rounded-[12px] shadow-card"
              />
            </div>

            <h2 className="m-0 max-w-[18ch] text-balance text-[clamp(30px,3.4vw,44px)] font-semibold leading-[1.14] tracking-[-0.026em] text-ink">
              {t("heading")}
            </h2>
            <p className="mt-4 max-w-[48ch] text-[15px] leading-[1.7] text-ink-2">
              {t("body")}
            </p>
          </div>

          <div className="lg:justify-self-end">
            <div className="flex w-full flex-col items-stretch gap-3 sm:w-auto sm:flex-row sm:items-center">
              <OsDownloadButton
                downloads={downloads}
                fallback={LATEST_RELEASE_URL}
                className="min-h-12 px-6 text-sm sm:min-w-[208px]"
                labels={downloadLabels(locale)}
              />
              <ButtonLink
                href="https://github.com/imrj05/orbit"
                variant="secondary"
                className="min-h-12 px-6 text-sm"
                external
              >
                <HugeiconsIcon icon={GithubIcon} data-icon="inline-start" />
                {t("github")}
              </ButtonLink>
            </div>

            <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 text-[13px] text-ink-3 lg:justify-end">
              {platforms.map(({ label, href }) => (
                <a
                  key={label}
                  href={href}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1.5 no-underline transition-colors hover:text-ink-2"
                >
                  <HugeiconsIcon
                    icon={Download01Icon}
                    className="size-3.5"
                    strokeWidth={1.8}
                  />
                  {label}
                </a>
              ))}
              {release ? (
                <Link
                  href={`/changelog/${release.tag}`}
                  className="no-underline transition-colors hover:text-ink-2"
                >
                  {t("releaseNotes", { tag: release.tag })}
                </Link>
              ) : null}
            </div>
          </div>
        </div>
      </Band>
    </Shell>
  );
}
