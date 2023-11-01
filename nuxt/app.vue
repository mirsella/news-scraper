<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db } = useNuxtApp();
const news = useState<News[]>("news", () => []);

onMounted(async () => {
  const jwt = window?.localStorage.getItem("jwt");
  if (!jwt) navigateTo("/login");

  try {
    await $db.wait();
    const liveQueryUuid = await $db?.live("news", ({ action, result }) => {
      switch (action) {
        case "CREATE":
          news.value.unshift(result as News);
          break;
        case "UPDATE":
          const index = news.value.findIndex((n) => n.id === result.id);
          if (index !== -1) news.value[index] = result as News;
          break;
        case "DELETE":
          const index2 = news.value.findIndex((n) => n.id === result.id);
          if (index2 !== -1) news.value.splice(index2, 1);
          break;
      }
    });
  } catch (e: any) {
    useToast().add({
      title: "Error starting live query",
      description: e.toString(),
      color: "red",
      timeout: 0,
    });
  }
});
</script>

<template>
  <Header />
  <NuxtPage />
  <ClientOnly>
    <UNotifications />
  </ClientOnly>
</template>
