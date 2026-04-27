import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: '#1c1916',
        mist: '#7a746e',
        line: 'rgba(28,22,18,0.08)',
        accent: '#8b5e3c',
        panel: '#ffffff',
        panelAlt: '#f8f5f1',
      },
      boxShadow: {
        chrome: '0 4px 16px rgba(28, 22, 18, 0.08)',
      },
      backgroundImage: {
        atmos:
          'radial-gradient(circle at top, rgba(139, 94, 60, 0.12), transparent 28%), radial-gradient(circle at 80% 20%, rgba(196, 133, 90, 0.08), transparent 24%), linear-gradient(180deg, #f2ede7 0%, #ece7e0 100%)',
      },
      fontFamily: {
        sans: ['Segoe UI', 'SF Pro Text', 'Inter', 'Avenir Next', 'sans-serif'],
      },
    },
  },
  plugins: [],
} satisfies Config
