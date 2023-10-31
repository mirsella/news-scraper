<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
const user = ref("");
const password = ref("");
const isLoading = ref(false);
const areFieldsValid = computed(() => {
  return user.value.length > 0 && password.value.length > 0;
});

async function signin() {
  isLoading.value = true;
  const toast = useToast();
  try {
    const jwt = await $db.signin({
      NS: "news",
      DB: "news",
      SC: "user",
      name: user.value,
      password: password.value,
    });
    if (!jwt) throw new Error("didn't get a jwt");
    window.localStorage.setItem("jwt", jwt);
    $dbhelper.update_activated();
    $dbhelper.authenticated.value = true;
    await navigateTo("/");
  } catch (error: any) {
    toast.add({
      color: "red",
      title:
        "sign in failed. probably wrong credentials or account doesn't exist",
      description: error.toString(),
    });
  }
  isLoading.value = false;
}
async function signup() {
  isLoading.value = true;
  const toast = useToast();
  try {
    let token = await $db.signup({
      NS: "news",
      DB: "news",
      SC: "user",
      name: user.value,
      password: password.value,
    });
    window.localStorage.setItem("jwt", token);
    $dbhelper.authenticated.value = true;
    toast.add({
      id: "signup_success",
      title: "Success, You have been signed up",
    });
    $dbhelper.update_activated();
  } catch (error: any) {
    toast.add({
      color: "red",
      title: "The sign up failed. name probably already taken",
      description: error.toString(),
    });
  }
  isLoading.value = false;
}
</script>

<template>
  <div class="p-4 flex justify-center">
    <ClientOnly fallback="Loading login page...">
      <UCard
        class="p-2 w-full md:w-1/2"
        v-if="$dbhelper?.connected.value === true"
        v-on:keyup.enter="signin"
      >
        <template #header>
          {{
            $route.query.expired === null
              ? "your connection has expired. please login again"
              : "You need to login to access this page"
          }}
        </template>
        <h1 v-if="user.length < 1" class="font-thin">
          Username needs to be at least 1 character
        </h1>
        <UInput
          class="mb-2"
          size="xl"
          placeholder="Username"
          v-model="user"
          :required="true"
          minlength="1"
          autofocus
        />
        <h1 v-if="password.length < 1" class="font-thin">
          Password needs to be at least 1 character
        </h1>
        <UInput
          class="mb-2"
          size="xl"
          placeholder="Password"
          v-model="password"
          type="password"
          :required="true"
          minlength="1"
        />
        <template #footer>
          <div class="w-full flex justify-around">
            <UButton
              id="signin"
              size="xl"
              :loading="isLoading"
              :disabled="!areFieldsValid"
              @click="signin"
              >Sign in</UButton
            >
            <UButton
              id="signup"
              size="xl"
              :loading="isLoading"
              :disabled="!areFieldsValid"
              @click="signup"
              >Sign up</UButton
            >
          </div>
        </template>
      </UCard>
    </ClientOnly>
  </div>
</template>
