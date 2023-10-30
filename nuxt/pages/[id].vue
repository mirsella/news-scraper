<script setup lang="ts">
import type { News } from "~/utils/news.d.ts";
const { $db } = useNuxtApp();
const newsstate = useState<News[]>("news", () => []);
const route = useRoute();
let news =
  newsstate.value.find((e: News) => e.id === route.params.id) || ({} as News);

if (process.client) {
  if (!newsstate.value.length) {
    await $db.wait();
    try {
      const res = await $db.query<[News]>(
        "select * from only news where id = $id",
        {
          id: route.params.id,
        },
      );
      if (!res[0].result) throw new Error("No news found");
      newsstate.value.push(res[0].result);
    } catch (error) {
      useToast().add({
        title: "Error querying news",
        description: error as string,
        timeout: 0,
      });
    }
  }
  news =
    newsstate.value.find((e: News) => e.id === route.params.id) || ({} as News);
}
</script>
<template>
  {{ news }}
  <div class="m-4">
    <NewsCard v-if="Object.keys(news).length > 0" :news="news" />
  </div>
</template>
