// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ["@nuxt/ui"],
  devtools: { enabled: true },
  app: {
    head: {
      titleTemplate: "%s - News-scraper",
    },
  },
  runtimeConfig: {
    public: {
      surrealdb_url: process.env.surrealdb_url || "http://127.0.0.1:8000",
    },
  },
  ui: {
    icons: ["carbon", "heroicons"],
  },
});
