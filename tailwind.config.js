module.exports = {
  content: {
    files: ["src/**/*.rs", "index.html"],
  },
  darkMode: "media", // 'media' or 'class'
  theme: {
    borderColor: ({ theme }) => ({
      ...theme('colors'),
      DEFAULT: theme('colors.stone.600')
    }),
    extend: {},
  },
  variants: {
    extend: {},
  },
  plugins: [],
};
