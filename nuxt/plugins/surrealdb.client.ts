import { Surreal } from "surrealdb.js";

const db = new Surreal();
const authenticated = ref(false);
const connected = ref(false);

let alreadyNotified = false;
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
    alreadyNotified = false;
  } catch (error) {
    const message = "Failed to connect to the database";
    if (alreadyNotified) return;
    alreadyNotified = true;
    useToast().add({
      title: "connection error",
      description: message,
      timeout: 10000,
    });
    // const errors = useState<string[]>("errors", () => []);
    // if (!errors.value.some((e) => e == message)) {
    //   errors.value.push(message);
    // }
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

export default defineNuxtPlugin(async () => {
  connect();
  login();
  return {
    provide: {
      db: db,
      dbhelper: {
        authenticated,
        connected,
        login,
        connect,
      },
    },
  };
});

setInterval(async () => {
  if (db.status !== 0) {
    connected.value = false;
    authenticated.value = false;
    connect();
  }
}, 1000);
