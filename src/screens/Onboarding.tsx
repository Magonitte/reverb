import { useTranslation } from "react-i18next";
import { VinylDisc } from "@/components/ui/VinylDisc";

/** `/onboarding`: tela cheia, sem navegação (os passos entram na F12). */
export default function Onboarding() {
  const { t } = useTranslation();
  return (
    <section
      aria-labelledby="screen-title"
      className="flex min-h-full flex-col items-center justify-center gap-6 p-8 text-center"
    >
      <VinylDisc size={180} spinning />
      <h1 id="screen-title" className="font-display text-[26px] font-semibold tracking-[-0.02em]">
        {t("onboarding.title")}
      </h1>
      <p className="max-w-[420px] text-sm leading-relaxed text-fg-muted">
        {t("onboarding.subtitle")}
      </p>
    </section>
  );
}
