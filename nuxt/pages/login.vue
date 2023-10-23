<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
const user = ref("");
const password = ref("");
const isLoading = ref(false);

async function signin() {
  isLoading.value = true;
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
      title: "Success, You have been signed up",
    });
  } catch (error: any) {
    toast.add({
      color: "red",
      title: "The sign up failed",
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
      >
        <template #header>
          {{
            $route.query.expired === null
              ? "your connection has expired. please login again"
              : "You need to login to access this page"
          }}
        </template>
        <UInput class="mb-2" size="xl" placeholder="Username" v-model="user" />
        <UInput
          class="mb-2"
          size="xl"
          placeholder="Password"
          v-model="password"
          type="password"
        />
        <template #footer>
          <div class="w-full flex justify-around">
            <UButton id="signin" size="xl" :loading="isLoading" @click="signin"
              >Sign in</UButton
            >
            <UButton id="signup" size="xl" :loading="isLoading" @click="signup"
              >Sign up</UButton
            >
          </div>
        </template>
      </UCard>
    </ClientOnly>
  </div>
</template>
