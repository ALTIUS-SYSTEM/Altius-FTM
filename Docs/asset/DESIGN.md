---
name: Light-theme
colors:
  surface: '#f5fafd'
  surface-dim: '#d5dbde'
  surface-bright: '#f5fafd'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#eff4f8'
  surface-container: '#e9eff2'
  surface-container-high: '#e3e9ec'
  surface-container-highest: '#dee3e7'
  on-surface: '#171c1f'
  on-surface-variant: '#3d494d'
  inverse-surface: '#2b3134'
  inverse-on-surface: '#ecf1f5'
  outline: '#6d797e'
  outline-variant: '#bcc8ce'
  surface-tint: '#00677e'
  primary: '#00677e'
  on-primary: '#ffffff'
  primary-container: '#0fb4da'
  on-primary-container: '#004151'
  inverse-primary: '#4fd6fd'
  secondary: '#3a6473'
  on-secondary: '#ffffff'
  secondary-container: '#bbe7f7'
  on-secondary-container: '#3e6877'
  tertiary: '#885200'
  on-tertiary: '#ffffff'
  tertiary-container: '#e3942c'
  on-tertiary-container: '#573300'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#b5ebff'
  primary-fixed-dim: '#4fd6fd'
  on-primary-fixed: '#001f28'
  on-primary-fixed-variant: '#004e60'
  secondary-fixed: '#bee9fa'
  secondary-fixed-dim: '#a2cdde'
  on-secondary-fixed: '#001f28'
  on-secondary-fixed-variant: '#204c5a'
  tertiary-fixed: '#ffddbb'
  tertiary-fixed-dim: '#ffb867'
  on-tertiary-fixed: '#2b1700'
  on-tertiary-fixed-variant: '#673d00'
  background: '#f5fafd'
  on-background: '#171c1f'
  surface-variant: '#dee3e7'
typography:
  display-lg:
    fontFamily: Geist
    fontSize: 48px
    fontWeight: '700'
    lineHeight: 56px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Geist
    fontSize: 32px
    fontWeight: '600'
    lineHeight: 40px
  headline-md:
    fontFamily: Geist
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  headline-sm:
    fontFamily: Geist
    fontSize: 20px
    fontWeight: '600'
    lineHeight: 28px
  body-lg:
    fontFamily: Public Sans
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  body-md:
    fontFamily: Public Sans
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Public Sans
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
  label-bold:
    fontFamily: Public Sans
    fontSize: 12px
    fontWeight: '600'
    lineHeight: 16px
    letterSpacing: 0.05em
  mono-data:
    fontFamily: Public Sans
    fontSize: 13px
    fontWeight: '500'
    lineHeight: 18px
rounded:
  sm: 0.5rem
  DEFAULT: 1rem
  md: 1.5rem
  lg: 2rem
  xl: 3rem
  full: 9999px
spacing:
  base: 4px
  container-margin: 24px
  gutter-lg: 24px
  gutter-md: 16px
  stack-sm: 8px
  stack-md: 16px
  stack-lg: 32px
---

## Brand & Style

The design system is engineered for professional logistics management, prioritizing speed, reliability, and operational clarity. The visual language conveys high-efficiency movement through modern typography and a structured, functional interface.

The design style is **Corporate / Modern** with a focus on data density and utilitarian precision. It utilizes a refined grid system to manage complex shipping information while maintaining a professional and trustworthy demeanor. The aesthetic avoids unnecessary decoration, opting instead for clear information hierarchy and high-contrast wayfinding. The palette and rounded geometry introduce a softer, more approachable feel to industrial efficiency, now optimized for a high-performance light mode environment.

## Colors

The system utilizes a **Light Mode** primary environment to ensure maximum readability and a clean, professional "paper-like" workspace for logistics coordinators. The palette is anchored by a refreshed **Bright Cyan** (#0FB4DA) for primary actions, providing a calm but distinct focus point. **Slate Blue** (#537D8C) serves as the core secondary color, primarily used for "Positive Action" states, success indicators, and highlighting progress in the shipping lifecycle.

**Amber Accent** (#E99931) is reserved for high-attention alerts, warnings, and secondary call-to-actions. Backgrounds use a crisp, neutral foundation to maintain professional focus while ensuring all data points remain sharp and legible in high-glare office environments.

## Typography

This design system uses a dual-font strategy. **Geist** is used for headlines and prominent UI markers to provide a modern, technical feel that reflects the precision of the brand. **Public Sans** is the workhorse for body text, data tables, and forms, selected for its exceptional legibility and neutral tone.

For data-heavy views like tracking numbers or SKU lists, utilize the `mono-data` style or Public Sans with tabular lining figures to ensure vertical alignment of numerical data.

## Layout & Spacing

The layout utilizes a **Fixed Grid** for internal dashboard content with a maximum container width of 1440px, centered on larger screens. A 12-column system is used for standard layouts, while complex data tables may adopt a fluid width approach to maximize information density.

**Desktop:** 12 columns, 24px gutters, 24px margins.
**Tablet:** 8 columns, 16px gutters, 16px margins.
**Mobile:** 4 columns, 12px gutters, 12px margins.

Spacing follows a 4px base unit to allow for tight, data-rich layouts typical of logistics management software. Vertical rhythms should prioritize grouping related input fields and shipping details using the `stack-sm` (8px) unit.

## Elevation & Depth

Hierarchy is established through **Tonal Layers** and subtle, functional depth cues appropriate for a light interface.

1.  **Floor (Level 0):** The primary light background.
2.  **Surface (Level 1):** Card containers with a subtle border or slightly darker tonal shift to define boundaries.
3.  **Raised (Level 2):** Subtle elevation for interactive elements like hover-state cards or dropdowns, indicated by a soft, diffused shadow.
4.  **Overlay (Level 3):** Modal windows and floating action menus with a pronounced shadow to pull the element forward.

Avoid heavy blurs; the depth should feel clean and architectural.

## Shapes

The design system adopts a **Pill-shaped** (Level 3) corner radius. This choice modernizes the interface, moving away from rigid industrial edges to a more fluid and contemporary software aesthetic.

- **Standard Buttons & Inputs:** 16px radius (`rounded`)
- **Cards & Primary Containers:** 32px radius (`rounded-lg`)
- **Status Badges & Chips:** 48px radius (`rounded-xl`) or fully rounded for a pill-style appearance.

The consistent 16px/32px logic ensures a clean, interlocking visual when components are placed side-by-side.

## Components

### Buttons
- **Primary:** Bright Cyan background with dark text. Used for "Create Shipment" or "Confirm."
- **Secondary:** Transparent background with Bright Cyan border and text.
- **Success:** Slate Blue background with light text.
- **Ghost:** No background, Bright Cyan or Amber Accent text.

### Inputs & Forms
- Input fields use a 1px border. On focus, the border thickens to 2px in Bright Cyan.
- Labels sit above the field in `label-bold` style for maximum clarity.
- Radii are set to 16px to match the pill-shaped theme.

### Data Tables (Critical Component)
- High-density rows (40px height).
- Alternating row stripes (Zebra striping) using light tonal variations of the surface color.
- Fixed headers for long lists of shipping manifests.
- Status columns must use "Slate Blue" for 'Delivered' and "Amber Accent" for 'In Transit' or 'Delayed'.

### Chips & Badges
- Used for shipment status (e.g., "Air Freight", "Express", "Pending"). 
- Use fully rounded (pill-style) shapes with colored backgrounds to maintain visibility on light surfaces.

### Cards
- White elevated surface background, 1px border, 32px corner radius.
- Used for summarizing shipment metrics (Total Orders, Pending Pickups, Revenue).