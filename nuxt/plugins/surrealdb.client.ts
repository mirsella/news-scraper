import { Surreal } from "surrealdb.js";

export default defineNuxtPlugin({
  name: "surrealdb",
  parallel: false,
  setup(NuxtApp) {
    const db = new Surreal();
    const authenticated = ref(false);

    return {
      provide: {
        db: db,
      },
    };
  },
});
