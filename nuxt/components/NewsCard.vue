<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db, $dbhelper } = useNuxtApp();

const BadgeCss = "m-1 min-h-8";
const props = defineProps<{
  news: News;
}>();
onMounted(async () => {
  await $db?.wait();
  if (!props.news.text_body || !props.news.html_body) {
    try {
      let ret: any = await $db.query(
        `select text_body, html_body from ${props.news.id}`,
      );
      console.log("return from querying body", ret);
      props.news.text_body = ret[0].result[0].text_body;
      props.news.html_body = ret[0].result[0].html_body;
    } catch (error: any) {
      useToast().add({
        title: "Error querying news",
        description: error.toString(),
        timeout: 0,
      });
    }
  }
});

async function updateNews() {
  await $db?.wait();
  try {
    const res = await $db?.merge<News>(props.news.id, props.news);
  } catch (error: any) {
    useToast().add({
      title: "Error saving news",
      description: error.toString(),
      timeout: 0,
    });
  }
}
</script>

<template>
  <UCard>
    <template #header>
      <div class="flex flex-wrap">
        <UBadge :class="BadgeCss">
          <UInput
            variant="none"
            v-model.number="news.rating"
            placeholder="rating"
            @vue:updated="updateNews"
            type="number"
            max="100"
            min="0"
            color="primary"
            class="w-[6.5rem]"
            :ui="{ trailing: { padding: { sm: 'pe-12' } } }"
          >
            <template #trailing>rating</template>
          </UInput>
        </UBadge>
        <UBadge :class="BadgeCss">
          {{ new Date(news.date).toLocaleString() }}
        </UBadge>
        <UBadge :class="BadgeCss">provider: {{ news.provider }}</UBadge>
        <UBadge :class="BadgeCss">
          <span class="mr-2">has been used</span>
          <UToggle color="emerald" v-model="news.used" class="ring" />
        </UBadge>
        <UBadge :class="BadgeCss">
          <a :href="news.link" target="_blank" rel="noopener noreferrer">
            {{ news.link }}
          </a>
        </UBadge>
      </div>
    </template>
    {{ news }}
  </UCard>
</template>
