<script setup lang="ts">
const { $db, dbhelper } = useNuxtApp();
import type { News } from "~/utils/news";
const queryStatus = ref("");

const news = useState<News[]>("news", () => []);
(async () => {
  queryStatus.value = "loading news";
  const t1 = performance.now();
  const result = await $db?.query<[News[]]>(
    "select * omit text_body, html_body from news",
  );
  if (result[0]?.status !== "OK") {
    useToast().add({
      title: "Error querying news",
      description: result.toString(),
      color: "red",
      timeout: 0,
    });
    return;
  }
  console.log(result);
  news.value = result[0]?.result ?? [];
  const t2 = performance.now();
  const dbtime = (result[0]?.time).replace(/\.[0-9]*/, "") || "unknown";
  queryStatus.value = `loaded ${news.value.length} news in ${
    t2 - t1
  }ms (db: ${dbtime})`;
})();
function updateurl() {
  navigateTo(Math.random().toString());
}
function removeurl() {
  navigateTo("/");
}
</script>

<template>
  <div>
    <NewsModal />
    <h1 class="text-lg font-bold w-full text-center">{{ queryStatus }}</h1>
    <div>
      <UButton @click="updateurl">update url</UButton>
      <UButton @click="removeurl">remove url</UButton>
    </div>
    <div>here will be the table</div>
  </div>
</template>
