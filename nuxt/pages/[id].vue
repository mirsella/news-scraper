<script setup lang="ts">
import type { News } from "~/utils/news.d.ts";
const { $db, $dbhelper } = useNuxtApp();
const newsstate = useState<News[]>("news", () => []);
const route = useRoute();
const notfound = ref(false);

const news = computed(() => {
  return newsstate.value.find((n) => n.id === route.params.id) || ({} as News);
});

onMounted(async () => {
  if (!newsstate.value.length) {
    while (!$dbhelper.authenticated.value || !$dbhelper.activated.value) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    try {
      const res = await $db.query<[News]>(
        "select * from only news where id = $id",
        {
          id: route.params.id,
        },
      );
      if (!res[0].rating) res[0].rating = -1;
      newsstate.value.unshift(res[0]);
    } catch (error) {
      notfound.value = true;
      useToast().add({
        title: "Error querying news",
        description: error as string,
        timeout: 0,
      });
    }
  }

  try {
    const liveQueryUuid = await $db?.live("news", ({ action, result }) => {
      if (!result) return;
      if (!result.rating) result.rating = -1;
      switch (action) {
        case "UPDATE":
          const index = newsstate.value.findIndex((n) => n.id === result.id);
          if (index !== -1) newsstate.value[index] = result as News;
          break;
        case "DELETE":
          useToast().add({
            title: "the news you are watching has been deleted.",
            color: "red",
            timeout: 0,
          });
          break;
      }
    });
    onUnmounted(async () => {
      await $db?.kill(liveQueryUuid);
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
  <div class="m-4">
    <ClientOnly>
      <NewsCard v-if="news?.id" :news="news" />
      <NotFound v-else />
    </ClientOnly>
  </div>
</template>
