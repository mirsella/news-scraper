<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
import type { News } from "~/utils/news";
const queryStatus = ref("");
const queryLoading = ref(true);

const news = useState<News[]>("news", () => []);
onMounted(async () => {
  while (!$dbhelper.authenticated.value || !$dbhelper.activated.value) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
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

const route = useRoute();
const isOpen = computed<boolean>({
  get: () => route?.query.id !== undefined && route?.query.id !== "",
  set: (value: boolean) => {
    if (!value) navigateTo({ query: {} });
  },
});
const n = computed<News>(() => {
  return news.value.find((n) => n.id === route?.query.id) || ({} as any);
});
const clipboardIcon = ref("i-heroicons-clipboard-document");
async function copyDedicatedLink() {
  const url = `${location.origin}/${route?.query.id}`;
  await navigator.clipboard.writeText(url);
  clipboardIcon.value = "i-heroicons-clipboard-document-check";
  setTimeout(() => {
    clipboardIcon.value = "i-heroicons-clipboard-document";
  }, 1000);
}
</script>

<template>
  <div>
    <ClientOnly>
      <UModal
        v-model="isOpen"
        :transition="false"
        :ui="{ width: 'md:max-w-[80%]' }"
      >
        <div class="flex justify-center">
          <UTooltip text="Open in a dedicated page">
            <UButton
              class="w-8 m-1 transition hover:scale-110"
              icon="i-carbon-export"
              @click="navigateTo('/' + $route.query.id)"
            />
          </UTooltip>
          <UTooltip text="Copy dedicated link to clipboard">
            <UButton
              class="w-8 m-1 transition hover:scale-110"
              :icon="clipboardIcon"
              @click="copyDedicatedLink"
            />
          </UTooltip>
        </div>
        <NewsCard :news="n" />
      </UModal>
    </ClientOnly>
    <ClientOnly>
      <h1 class="text-lg font-bold w-full text-center">{{ queryStatus }}</h1>
    </ClientOnly>
    <NewsTable :loading="queryLoading" class="mx-4" />
  </div>
</template>
