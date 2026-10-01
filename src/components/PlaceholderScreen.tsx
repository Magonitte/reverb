import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/ui/EmptyState";
import { ScreenHeader } from "./ScreenHeader";

export interface PlaceholderScreenProps {
  /** Namespace i18n da tela: lê `<ns>.title`, `<ns>.subtitle`, `<ns>.empty.title`, `<ns>.empty.description`. */
  ns: string;
  icon: LucideIcon;
  children?: ReactNode;
}

/** Tela com título, subtítulo e estado vazio. As fases seguintes substituem o miolo. */
export function PlaceholderScreen({ ns, icon: Icon, children }: PlaceholderScreenProps) {
  const { t } = useTranslation();
  return (
    <section aria-labelledby="screen-title">
      <ScreenHeader title={t(`${ns}.title`)} subtitle={t(`${ns}.subtitle`)} />
      {children}
      <EmptyState
        icon={<Icon aria-hidden="true" />}
        title={t(`${ns}.empty.title`)}
        description={t(`${ns}.empty.description`)}
      />
    </section>
  );
}
