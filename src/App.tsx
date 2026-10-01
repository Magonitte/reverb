import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppInfo } from "@/bindings/AppInfo";
import { api } from "@/lib/ipc/api";

export function App() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    let alive = true;
    api.appInfo().then((value) => {
      if (alive) setInfo(value);
    });
    return () => {
      alive = false;
    };
  }, []);

  return (
    <main className="p-6">
      <h1 className="text-2xl font-semibold">{t("app.name")}</h1>
      {info ? (
        <p data-testid="app-info">
          {info.version} · {info.platform}
        </p>
      ) : (
        <p>{t("app.loading")}</p>
      )}
    </main>
  );
}
