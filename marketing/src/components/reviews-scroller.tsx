"use client";

import { useState, type ReactNode } from "react";

/**
 * The reviews wall moves indefinitely. This wrapper adds the visible pause
 * control the motion needs; hover and keyboard focus still pause it in CSS.
 */
export function ReviewsScroller({
  label,
  pauseLabel,
  resumeLabel,
  head,
  children,
}: {
  label: string;
  pauseLabel: string;
  resumeLabel: string;
  /** The section's heading, set on the same row as the pause control. */
  head?: ReactNode;
  children: ReactNode;
}) {
  const [paused, setPaused] = useState(false);

  return (
    <div className="relative">
      <div className="flex flex-col gap-6 sm:flex-row sm:items-end sm:justify-between">
        {head}
        <button
          type="button"
          onClick={() => setPaused((value) => !value)}
          aria-pressed={paused}
          className="min-h-[44px] shrink-0 self-start rounded-[10px] border border-edge-default bg-surface-1 px-4 py-2 text-[13px] text-ink-2 sm:self-auto no-underline transition-colors hover:border-edge-strong hover:bg-surface-2 hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
        >
          {paused ? resumeLabel : pauseLabel}
        </button>
      </div>
      <div
        className="reviews-scroller mt-10"
        data-paused={paused ? "true" : undefined}
        role="region"
        aria-label={label}
      >
        {children}
      </div>
    </div>
  );
}
