<script setup lang="ts">
import type { News } from "~/utils/news.d.ts";
const { $db } = useNuxtApp();
const newsstate = useState<News[]>("news", () => []);
const route = useRoute();

const news = computed(() => {
  return newsstate.value.find((n) => n.id === route.params.id) || ({} as News);
});

onMounted(async () => {
  if (!newsstate.value.length) {
    await $db.wait();
    try {
      const res = await $db.query<[News]>(
        "select * from only news where id = $id",
        {
          id: route.params.id,
        },
      );
      console.log("getting news on [id]", res);
      newsstate.value.unshift(res[0]);
    } catch (error) {
      useToast().add({
        title: "Error querying news",
        description: error as string,
        timeout: 0,
      });
    }
  }

  try {
    const liveQueryUuid = await $db?.live("news", ({ action, result }) => {
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
      <div v-else class="text-center text-4xl w-full">
        no news found for this id.
        <UButton label="go home." size="xl" class @click="navigateTo('/')" />
      </div>
    </ClientOnly>
  </div>
</template>
