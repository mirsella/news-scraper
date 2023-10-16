import { Surreal } from "surrealdb.js";

export default defineNuxtPlugin({
  name: "surrealdb",
  parallel: true,
  async setup(NuxtApp) {
    const db = new Surreal();
    NuxtApp.provide("db", db);
  },
});
