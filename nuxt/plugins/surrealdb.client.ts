import { Surreal } from "surrealdb.js";

const db = new Surreal();
const authenticated = ref(false);
const connected = ref(false);
const activated = ref(false);

let alreadyFailed = false;
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
    alreadyFailed = false;
  } catch (error) {
    const message = "Failed to connect to the database";
    if (alreadyFailed) return;
    alreadyFailed = true;
    useToast().add({
      title: "connection error",
      description: message,
      timeout: 10000,
    });
  }
}

async function login(): Promise<Boolean> {
  const jwt = localStorage.getItem("jwt");
  if (!jwt) return false;
  try {
    const auth = await db.authenticate(jwt);
    if (auth) {
      authenticated.value = true;
      update_activated();
      return true;
    } else {
      throw new Error("Failed to authenticate");
    }
  } catch (error: any) {
    authenticated.value = false;
    window.localStorage.removeItem("jwt");
    useToast().add({
      color: "red",
      title: "Authentication failed",
      description: error.toString(),
    });
    navigateTo("/login?expired");
    return false;
  }
}

async function update_activated() {
  try {
    const user: any = await db.info();
    if (user.activated) {
      activated.value = true;
    } else {
      throw new Error("User not activated");
    }
  } catch (error) {
    activated.value = false;
    useToast().add({
      id: "activation_notice",
      color: "red",
      title: "your account needs to be activated",
      description: "please contact the administrator to have access.",
      timeout: 0,
    });
  }
}

export default defineNuxtPlugin(async () => {
  (async () => {
    await connect();
    setInterval(async () => {
      if (db.status !== 0 && alreadyFailed === true) {
        connected.value = false;
        authenticated.value = false;
        connect();
      }
    }, 100);
    if (connected.value) await login();
  })();
  return {
    provide: {
      db: db,
      dbhelper: {
        authenticated,
        connected,
        activated,
        login,
        connect,
        update_activated,
      },
    },
  };
});
