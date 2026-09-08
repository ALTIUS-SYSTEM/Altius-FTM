import en from './en.json';
import id from './id.json';
import th from './th.json';
import ja from './ja.json';
import zh from './zh.json';
import filPH from './fil-PH.json';
import vi from './vi.json';

export const locales = ['en', 'id', 'th', 'ja', 'zh', 'fil-PH', 'vi'] as const;
export type Locale = typeof locales[number];
export type TranslationKey = keyof typeof en;
export type Catalog = Readonly<Record<TranslationKey, string>>;
export const catalogs: Readonly<Record<Locale, Catalog>> = Object.freeze({ en: Object.freeze(en), id: Object.freeze(id), th: Object.freeze(th), ja: Object.freeze(ja), zh: Object.freeze(zh), 'fil-PH': Object.freeze(filPH), vi: Object.freeze(vi) });
export const localeLabels: Readonly<Record<Locale, string>> = Object.freeze({ en: 'English', id: 'Bahasa Indonesia', th: 'ไทย', ja: '日本語', zh: '中文', 'fil-PH': 'Filipino', vi: 'Tiếng Việt' });
export function resolveLocale(value: string): Locale {
  const normalized = value.toLowerCase().replaceAll('_', '-');
  if (normalized === 'fil' || normalized.startsWith('fil-')) return 'fil-PH';
  return locales.find(locale => locale.toLowerCase() === normalized || locale === normalized.split('-')[0]) ?? 'en';
}
export function translate(locale: string, key: TranslationKey): string {
  return catalogs[resolveLocale(locale)][key] ?? catalogs.en[key];
}
export const i18nextResources = Object.fromEntries(locales.map(locale => [locale, { translation: catalogs[locale] }]));
