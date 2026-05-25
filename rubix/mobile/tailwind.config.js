/** Tailwind config for NativeWind v4. Tokens mirror the rubix
 * violet-bloom palette so the demo screen has a coherent look. Long
 * term: generate this file from `@nube/starter-theme-tokens` so web
 * shadcn + native gluestack share one source of truth. */
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,jsx,ts,tsx}'],
  presets: [require('nativewind/preset')],
  theme: {
    extend: {
      colors: {
        // Mirrors the violet-bloom named palette (light mode) so the
        // demo screen reads the same as the rest of the kit.
        background: '#FFFFFF',
        foreground: '#0B1220',
        card: '#FFFFFF',
        'card-foreground': '#0B1220',
        muted: '#F1F5F9',
        'muted-foreground': '#64748B',
        primary: '#7C3AED', // violet-600
        'primary-foreground': '#FFFFFF',
        accent: '#F3E8FF', // violet-100
        'accent-foreground': '#5B21B6',
        border: '#E5E7EB',
        ring: '#7C3AED',
        destructive: '#DC2626',
        success: '#16A34A',
        warning: '#F59E0B',
      },
      borderRadius: {
        lg: '14px',
        xl: '20px',
        '2xl': '24px',
        '3xl': '28px',
      },
      fontFamily: {
        sans: ['System'],
      },
    },
  },
  plugins: [],
};
