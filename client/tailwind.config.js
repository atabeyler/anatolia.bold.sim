/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        sim: {
          bg:      '#030310',
          surface: '#07071a',
          border:  '#3a0a0a',
          accent:  '#cc2222',
          gold:    '#d4a838',
          red:     '#e05a5a',
          green:   '#4ecb71',
          cyan:    '#00d4ff',
          text:    '#e0e0f0',
          muted:   '#8898c8',
        },
      },
      fontFamily: {
        mono:       ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
        orbitron:   ['Orbitron', 'monospace'],
        'share-tech': ['Share Tech Mono', 'monospace'],
        ui:         ['Inter', 'system-ui', 'sans-serif'],
      },
      animation: {
        'ring-expand': 'ring-expand 2.5s ease-out infinite',
        'ring-rotate': 'ring-rotate 14s linear infinite',
        'ring-rotate-rev': 'ring-rotate-rev 20s linear infinite',
        'hud-scan': 'hud-scan 3s ease-in-out infinite',
      },
      keyframes: {
        'ring-expand': {
          '0%':   { transform: 'scale(0.8)', opacity: '0.6' },
          '100%': { transform: 'scale(2.4)', opacity: '0' },
        },
      },
      boxShadow: {
        'neon-sm':  '0 0 8px rgba(79,110,247,0.5), 0 0 20px rgba(79,110,247,0.2)',
        'neon-md':  '0 0 15px rgba(79,110,247,0.6), 0 0 40px rgba(79,110,247,0.3)',
        'neon-lg':  '0 0 25px rgba(79,110,247,0.7), 0 0 60px rgba(79,110,247,0.4), 0 0 100px rgba(79,110,247,0.15)',
        'neon-cyan': '0 0 15px rgba(0,212,255,0.6), 0 0 40px rgba(0,212,255,0.3)',
        'neon-gold': '0 0 15px rgba(212,168,56,0.6), 0 0 40px rgba(212,168,56,0.3)',
      },
    },
  },
  plugins: [],
};
