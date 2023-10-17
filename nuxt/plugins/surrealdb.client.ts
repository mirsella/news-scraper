import { Surreal } from "surrealdb.js";

/*
if no token is found, we need to show a login component
*/

const db = new Surreal();
const authenticated = ref(false);

async function connect() {
  const surrealdb_url = process.env.surrealdb_url;
  if (!surrealdb_url) {
    throw new Error("surrealdb_url is not set");
  }
  return db.connect(surrealdb_url, {
    ns: "news",
    db: "news",
  });
}

async function login() {
  const jwt = localStorage.getItem("jwt");
  if (!jwt) {
    // TODO: show login page
    return;
  }
  const authenticated = await db.authenticate(jwt);
  if (authenticated) {
    // TODO: show succesfull login
  } else {
    // TODO: show login page "connection expired"
  }
}

export default defineNuxtPlugin({
  name: "surrealdb",
  parallel: true,
  async setup(NuxtApp) {
    return {
      provide: {
        db: db,
        dbhelper: {
          authenticated,
          login,
        },
      },
    };
  },
});
