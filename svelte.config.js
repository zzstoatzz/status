import adapter from "@sveltejs/adapter-node";

export default {
  kit: {
    adapter: adapter(),
    files: {
      src: "app",
    },
    alias: {
      $hatk: "./hatk.generated.ts",
      "$hatk/client": "./hatk.generated.client.ts",
    },
  },
};
