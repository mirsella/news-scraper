import { Surreal } from "surrealdb.js";

const db = new Surreal({
  onClose() {
    connected.value = false;
    authenticated.value = false;
    activated.value = false;
  },
});
const authenticated = ref(false);
const connected = ref(false);
const activated = ref(false);

let alreadyFailed = false;
async function connect() {
  connected.value = false;
  const config = useRuntimeConfig();
  let urls = config.public.surrealdb_urls.split(",");
  if (!urls.length) {
    useToast().add({
      title: "misconfiguration",
      description: "no surrealdb urls provided",
      color: "red",
      timeout: 0,
    });
  }
  try {
    console.log("testing urls", urls);
    let fetchPromises = urls.map(
      (url) =>
        new Promise((resolve) => {
          fetch(url, { method: "HEAD", redirect: "manual" })
            .then(() => resolve(url))
            .catch(() => null);
        }),
    );

    const url: any = await Promise.race(fetchPromises);
    console.log("fastest url is", url);
    await db.connect(url, {
      namespace: "news",
      database: "news",
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
      color: "red",
      timeout: 0,
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
    if (connected.value) {
      const ret = await login();
      if (ret) navigateTo("/");
    }
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
