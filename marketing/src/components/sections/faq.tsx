import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { JsonLd } from "@/components/json-ld";
import { SectionHead } from "@/components/sections/section-head";
import { Band, Shell } from "@/components/ui";
import { getT } from "@/i18n/t";
import { localeHref } from "@/i18n/links";
import type { Locale } from "@/i18n/config";
import { SITE } from "@/lib/site";

/**
 * Native `<details>` disclosures — no JavaScript, keyboard accessible, and the
 * answer text is in the initial HTML for crawlers.
 */
export async function Faq({ locale }: { locale: Locale }) {
  const t = getT(locale, "faq");
  const items = t.raw("items") as { q: string; a: string }[];
  /**
   * `FAQPage` structured data, built from the same items the section renders so
   * they can never fall out of sync — and so the schema is translated with the
   * page it describes. Eligible for FAQ rich results. The `@id` is scoped to
   * this page's URL: every language ships its own node, instead of ten
   * translated documents all claiming the English homepage's.
   */
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    "@id": `${SITE.url}${localeHref(locale, "/")}#faq`,
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.q,
      acceptedAnswer: { "@type": "Answer", text: item.a },
    })),
  };

  return (
    <Shell>
      <Band id="faq" dashed className="scroll-mt-20 py-20 sm:py-28">
        <JsonLd data={jsonLd} />

        <div className="grid grid-cols-1 gap-10 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-14">
          <SectionHead eyebrow={t("label")} title={t("heading")} copy={t("body")} />

          <div>
            {items.map((item) => (
              <details
                key={item.q}
                className="group border-b border-edge-subtle first:border-t"
              >
                <summary className="flex cursor-pointer list-none items-center justify-between gap-5 min-h-11 py-5 text-left text-[16px] font-medium text-ink transition-colors duration-150 hover:text-ink-2 [&::-webkit-details-marker]:hidden">
                  {item.q}
                  <HugeiconsIcon
                    icon={ArrowDown01Icon}
                    className="size-4 shrink-0 text-ink-3 transition-transform duration-200 group-open:rotate-180"
                    strokeWidth={1.8}
                  />
                </summary>
                <p className="m-0 max-w-[62ch] pb-6 pr-8 text-[15px] leading-[1.75] text-ink-2">
                  {item.a}
                </p>
              </details>
            ))}
          </div>
        </div>
      </Band>
    </Shell>
  );
}
