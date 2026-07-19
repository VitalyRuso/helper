/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#172026",
        paper: "#f7f6f0",
        brand: "#176b62",
        saffron: "#d99a2b",
        clay: "#b95343",
      },
      boxShadow: {
        soft: "0 18px 50px rgba(23, 32, 38, 0.12)",
      },
    },
  },
  plugins: [],
};
