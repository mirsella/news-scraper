<script setup lang="ts">
import type { News } from "~/utils/news.d.ts";
const { $db } = useNuxtApp();
const newsstate = useState<News[]>("news", () => []);
const route = useRoute();

const news = computed(() => {
  return newsstate.value.find((n) => n.id === route.params.id);
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
      if (!res[0].result) throw new Error("No news found");
      newsstate.value.unshift(res[0].result);
    } catch (error) {
      useToast().add({
        title: "Error querying news",
        description: error as string,
        timeout: 0,
      });
    }
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
