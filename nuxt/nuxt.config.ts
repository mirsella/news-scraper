// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ["@nuxt/ui"],
  app: {
    head: {
      titleTemplate: "%s gusnews",
    },
  },
  runtimeConfig: {
    public: {
      surrealdb_urls: process.env.SURREALDB_URLS || "",
    },
  },
  ui: {
    icons: ["carbon", "heroicons"],
  },
});
