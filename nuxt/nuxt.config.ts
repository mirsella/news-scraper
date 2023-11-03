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
      surrealdb_urls: [
        "http://news-scraper-db.loca.lt",
        "http://192.168.10.119:8000",
      ],
    },
  },
  ui: {
    icons: ["carbon", "heroicons"],
  },
  ssr: false,
});
