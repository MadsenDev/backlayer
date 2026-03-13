import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: '#070b11',
        mist: '#95a3ba',
        line: 'rgba(255,255,255,0.08)',
        accent: '#6ef2c8',
        panel: '#0d1524',
        panelAlt: '#121d31',
      },
      boxShadow: {
        chrome: '0 24px 80px rgba(0, 0, 0, 0.45)',
      },
      backgroundImage: {
        atmos:
          'radial-gradient(circle at top, rgba(95, 157, 255, 0.18), transparent 28%), radial-gradient(circle at 80% 20%, rgba(110, 242, 200, 0.1), transparent 24%), linear-gradient(180deg, #0b0f16 0%, #05070a 100%)',
      },
      fontFamily: {
        sans: ['Segoe UI', 'SF Pro Text', 'Inter', 'Avenir Next', 'sans-serif'],
      },
    },
  },
  plugins: [],
} satisfies Config
