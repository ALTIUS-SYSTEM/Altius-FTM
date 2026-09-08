/**
 * Supported documentation locales.
 */
export const LOCALES = ["en", "id"] as const;

/** Locale code type for the developer portal. */
export type Locale = (typeof LOCALES)[number];

/** Default docs locale (also used for `/` redirect). */
export const DEFAULT_LOCALE: Locale = "en";

/**
 * Returns true when `value` is a supported locale.
 */
export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value);
}
