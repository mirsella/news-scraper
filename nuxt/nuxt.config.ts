// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ["@nuxt/ui"],
  app: {
    head: {
      titleTemplate: "%s news-scraper",
    },
  },
  runtimeConfig: {
    public: {
      // surrealdb_url: process.env.SURREALDB_URL || "http://127.0.0.1:8000",
      surrealdb_url: undefined,
    },
  },
  ui: {
    icons: ["carbon", "heroicons"],
  },
});
