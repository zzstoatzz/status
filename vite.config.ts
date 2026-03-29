import { sveltekit } from "@sveltejs/kit/vite";
import { hatk } from "@hatk/hatk/vite-plugin";
import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [hatk(), sveltekit()],
  lint: {
    ignorePatterns: ["hatk.generated.ts", "hatk.generated.client.ts"],
  },
  fmt: {
    ignorePatterns: ["hatk.generated.ts", "hatk.generated.client.ts"],
  },
});
