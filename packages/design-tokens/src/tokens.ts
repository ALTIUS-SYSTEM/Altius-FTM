import source from './tokens.json';

export const colors = Object.freeze(source.colors);
export const typography = Object.freeze(Object.fromEntries(Object.entries(source.typography).map(([key, value]) => [key, Object.freeze(value)]))) as Readonly<{ [K in keyof typeof source.typography]: Readonly<typeof source.typography[K]> }>;
export const rounded = Object.freeze(source.rounded);
export const spacing = Object.freeze(source.spacing);
export const tokens = Object.freeze({ colors, typography, rounded, spacing, layout: Object.freeze(source.layout) });
