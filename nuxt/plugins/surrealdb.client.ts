import { Surreal } from "surrealdb.js";

const db = new Surreal();
const authenticated = ref(false);
const connected = ref(false);
let surrealdb_url: string;

async function connect() {
  await db.connect(surrealdb_url, {
    ns: "news",
    db: "news",
  });
}

async function login(): Promise<Boolean> {
  const jwt = localStorage.getItem("jwt");
  if (!jwt) return false;
  const auth = await db.authenticate(jwt);
  if (auth) {
    authenticated.value = true;
    return true;
  } else {
    authenticated.value = false;
    navigateTo("/login?expired");
    return false;
  }
}

export default defineNuxtPlugin(async (NuxtApp) => {
  const config = useRuntimeConfig();
  surrealdb_url = config.public.surrealdb_url;
  connect();
  login();
  return {
    provide: {
      db: db,
      dbhelper: {
        authenticated,
        connected,
        login,
      },
    },
  };
});
