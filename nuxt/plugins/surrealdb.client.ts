import { Surreal } from "surrealdb.js";

const db = new Surreal();
const authenticated = ref(false);
const connected = ref(false);

async function connect() {
  connected.value = false;
  const config = useRuntimeConfig();
  const db_url: string =
    config.public.surrealdb_url ||
    window.location.protocol + "//" + window.location.hostname + ":8000";
  try {
    await db.connect(db_url, {
      ns: "news",
      db: "news",
    });
    connected.value = true;
  } catch (error) {
    const errors = useState<string[]>("errors", () => []);
    errors.value.push("Failed to connect to the database");
  }
}

async function login(): Promise<Boolean> {
  const jwt = localStorage.getItem("jwt");
  if (!jwt) return false;
  const auth = await db.authenticate(jwt);
  if (auth) {
    authenticated.value = true;
    console.log("Authenticated");
    return true;
  } else {
    authenticated.value = false;
    navigateTo("/login?expired");
    return false;
  }
}

export default defineNuxtPlugin(async (NuxtApp) => {
  connect();
  login();
  NuxtApp.provide("db", db);
  NuxtApp.provide("dbhelper", {
    authenticated,
    connected,
    login,
  });
});
