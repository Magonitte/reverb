import { useEffect, useState } from "react";
import { api } from "@/lib/ipc/api";
import { VinylDisc } from "@/components/ui/VinylDisc";

export function LibraryCover({
  id,
  updatedAt,
  missing,
}: {
  id: number;
  updatedAt: number;
  missing: boolean;
}) {
  const [image, setImage] = useState<{ key: string; src: string | null } | null>(null);
  const key = `${id}:${updatedAt}`;
  useEffect(() => {
    let active = true;
    if (!missing) {
      api
        .libraryCover(id, 64)
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
  }, [id, key, missing]);
  const src = !missing && image?.key === key ? image.src : null;
  return src ? (
    <img src={src} alt="" className="size-9 rounded-md object-cover" />
  ) : (
    <VinylDisc size={36} />
  );
}
