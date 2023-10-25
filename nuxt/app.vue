<script setup lang="ts">
if (process.client) {
  const jwt = window?.localStorage.getItem("jwt");
  if (jwt) navigateTo("/");
  else navigateTo("/login");
}
</script>

<template>
  <Header />
  <NuxtPage />
  <ClientOnly>
    <UNotifications />
  </ClientOnly>
</template>
