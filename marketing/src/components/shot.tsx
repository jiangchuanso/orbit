"use client";

import Image from "next/image";
import { useEffect, useId, useRef } from "react";

import { useLightbox } from "@/components/lightbox";

/**
 * A product screenshot. The PNGs already carry their own window chrome and a
 * transparent gutter, so the image is rendered bare — no card frame, border,
 * padding, or shadow around it.
 */
export function Shot({
  src,
  srcLight,
  alt,
  width = 3164,
  height = 2068,
  priority = false,
  sizes = "(max-width: 768px) 100vw, (max-width: 1280px) 92vw, 1180px",
  className = "",
  framed = false,
}: {
  src: string;
  srcLight?: string;
  alt: string;
  width?: number;
  height?: number;
  priority?: boolean;
  sizes?: string;
  className?: string;
  /**
   * A close crop of the window (no chrome of its own) gets a window's edge: a
   * hairline, the product radius, and a fade over the side its text runs off.
   */
  framed?: boolean;
}) {
  const { register, open } = useLightbox();
  const id = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const el = buttonRef.current;
    if (!el) return;
    return register({ id, src, srcLight, alt, width, height }, el);
  }, [register, id, src, srcLight, alt, width, height]);

  return (
    <div className={className}>
      <button
        ref={buttonRef}
        type="button"
        onClick={() => open(id)}
        className={`block w-full cursor-zoom-in${
          framed ? " shot-framed rounded-[16px] border border-edge-default bg-surface-1" : ""
        }`}
      >
        <Image
          src={src}
          alt={alt}
          width={width}
          height={height}
          priority={priority}
          sizes={sizes}
          className={`h-auto w-full${srcLight ? " light:hidden" : ""}`}
        />
        {srcLight ? (
          <Image
            src={srcLight}
            alt={alt}
            width={width}
            height={height}
            priority={priority}
            sizes={sizes}
            className="hidden h-auto w-full light:block"
          />
        ) : null}
      </button>
    </div>
  );
}
