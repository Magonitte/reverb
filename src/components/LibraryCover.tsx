import { useEffect, useState } from "react";
import { api } from "@/lib/ipc/api";
import { VinylDisc } from "@/components/ui/VinylDisc";

export function LibraryCover({
  id,
  updatedAt,
  missing,
  size = 36,
}: {
  id: number;
  updatedAt: number;
  missing: boolean;
  size?: number;
}) {
  const [image, setImage] = useState<{ key: string; src: string | null } | null>(null);
  const key = `${id}:${updatedAt}:${size}`;
  useEffect(() => {
    let active = true;
    if (!missing) {
      api
        .libraryCover(id, Math.max(64, size * 2))
        .then((src) => {
          if (active) setImage({ key, src });
        })
        .catch(() => {
          if (active) setImage({ key, src: null });
        });
    }
    return () => {
      active = false;
    };
  }, [id, key, missing, size]);
  const src = !missing && image?.key === key ? image.src : null;
  return src ? (
    <img
      src={src}
      alt=""
      onError={() => setImage({ key, src: null })}
      style={{ width: size, height: size }}
      className="shrink-0 rounded-md object-cover"
    />
  ) : (
    <VinylDisc size={size} />
  );
}
