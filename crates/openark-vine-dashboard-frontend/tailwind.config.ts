import type { Config } from "tailwindcss";
import daisyui from "daisyui";

export default {
  content: ["./index.html", "./src/**/*.rs"],
  theme: {},
  plugins: [daisyui],
} satisfies Config;
