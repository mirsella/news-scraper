// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ["@nuxt/ui"],
  app: {
    head: {
      titleTemplate: "%s news-scraper",
    },
  },
  ui: {
    icons: ["carbon", "heroicons"],
  },
  ssr: false,
});
