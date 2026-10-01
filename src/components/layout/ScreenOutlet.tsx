import { Suspense } from "react";
import { Outlet, useLocation } from "react-router";
import { motion, useReducedMotion } from "motion/react";
import { HealBanner } from "@/components/HealBanner";
import { Skeleton } from "@/components/ui/Skeleton";

/** Área de conteúdo: fade + 8 px em 240 ms (sem deslocamento com prefers-reduced-motion). */
export function ScreenOutlet() {
  const { pathname } = useLocation();
  const reduce = useReducedMotion();
  // A chave é o 1º segmento: trocar de aba em /settings/:tab não reanima a tela.
  const key = `/${pathname.split("/")[1] ?? ""}`;
  return (
    <motion.div
      key={key}
      data-testid="screen"
      data-route={key}
      initial={{ opacity: 0, y: reduce ? 0 : 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: reduce ? 0 : 0.24, ease: [0.16, 1, 0.3, 1] }}
      className="p-8 max-sm:p-4"
    >
      <HealBanner />
      <Suspense
        fallback={
          <div data-testid="screen-loading" className="flex flex-col gap-3" aria-hidden="true">
            <Skeleton className="h-8 w-64" />
            <Skeleton className="h-4 w-96 max-w-full" />
            <Skeleton className="h-48 w-full" />
          </div>
        }
      >
        <Outlet />
      </Suspense>
    </motion.div>
  );
}
