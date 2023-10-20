<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
const user = ref("");
const password = ref("");
const isLoading = ref(false);

function signin() {
  isLoading.value = true;
  isLoading.value = false;
}
function signup() {
  isLoading.value = true;
  isLoading.value = false;
}
</script>

<template>
  <div class="p-2 w-full">
    <UCard v-if="$dbhelper.connected === true">
      <template #header>You need to login to access this page</template>
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
          <UButton id="signup" size="xl" :loading="isLoading" @click="signup">Sign up</UButton>
        </div>
      </template>
    </UCard>
  </div>
</template>
