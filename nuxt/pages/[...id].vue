<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
import type { News } from "~/utils/news";
const queryStatus = ref("");
const queryLoading = ref(true);

const news = useState<News[]>("news", () => []);
(async () => {
  queryStatus.value = "";
  queryLoading.value = true;
  await $db?.ready;
  try {
    const t1 = performance.now();
    const result = await $db?.query<[News[]]>(
      "select * omit text_body, html_body from news order by date desc",
    );
    if (!result || !result.length || result[0]?.status !== "OK") {
      throw new Error(result.toString());
    }
    news.value = result[0]?.result ?? [];
    const t2 = performance.now();
    const dbtime = (result[0]?.time).replace(/\.[0-9]*/, "") || "unknown";
    const totaltime = (t2 - t1).toFixed(0);
    queryStatus.value = `loaded ${news.value.length} news in ${totaltime}ms (db: ${dbtime})`;
    queryLoading.value = false;
  } catch (e: any) {
    if (process.server) return;
    useToast().add({
      title: "Error querying news",
      description: e.toString(),
      color: "red",
      timeout: 0,
    });
  }
})();

if (process.client) {
  try {
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
}
</script>

<template>
  <div>
    <NewsModal />
    <ClientOnly>
      <h1 class="text-lg font-bold w-full text-center">{{ queryStatus }}</h1>
    </ClientOnly>
    <NewsTable :loading="queryLoading" class="m-2" />
  </div>
</template>
