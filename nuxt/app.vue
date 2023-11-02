<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db, $dbhelper } = useNuxtApp();
const news = useState<News[]>("news", () => []);

onMounted(async () => {
  const jwt = window?.localStorage.getItem("jwt");
  if (!jwt) navigateTo("/login");
});
</script>

<template>
  <Header />
  <NuxtPage />
  <ClientOnly>
    <UNotifications />
  </ClientOnly>
</template>
