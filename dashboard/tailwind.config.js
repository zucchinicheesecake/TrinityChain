/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        'neon-pink': '#ff00ff',
        'neon-cyan': '#00ffff',
        'dark-purple': '#1a001a',
      },
      boxShadow: {
        'neon-glow': '0 0 5px #ff00ff, 0 0 10px #ff00ff, 0 0 20px #ff00ff',
      },
    },
  },
  plugins: [],
};
