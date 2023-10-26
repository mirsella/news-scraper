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
  const t1 = performance.now();
  const result = await $db?.query<[News[]]>(
    "select * omit text_body, html_body from news order by date desc",
  );
  if (!result || !result.length) return;
  if (result[0]?.status !== "OK") {
    useToast().add({
      title: "Error querying news",
      description: result.toString(),
      color: "red",
      timeout: 0,
    });
    return;
  }
  news.value = result[0]?.result ?? [];
  const t2 = performance.now();
  const dbtime = (result[0]?.time).replace(/\.[0-9]*/, "") || "unknown";
  const totaltime = (t2 - t1).toFixed(0);
  queryStatus.value = `loaded ${news.value.length} news in ${totaltime}ms (db: ${dbtime})`;
  queryLoading.value = false;
})();
</script>

<template>
  <div>
    <NewsModal />
    <h1 class="text-lg font-bold w-full text-center">{{ queryStatus }}</h1>
    <NewsTable
      :loading="queryLoading"
      class="m-2"
      v-if="$dbhelper && $dbhelper.authenticated.value"
    />
  </div>
</template>
