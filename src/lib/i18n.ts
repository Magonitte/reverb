import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "@/locales/en/translation.json";
import ptBR from "@/locales/pt-BR/translation.json";

void i18n.use(initReactI18next).init({
  resources: {
    "pt-BR": { translation: ptBR },
    en: { translation: en },
  },
  lng: "pt-BR",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
