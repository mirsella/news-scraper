<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db, $dbhelper } = useNuxtApp();

let props = defineProps<{
  news: News;
}>();
watch(
  () => props.news,
  async () => {
    if (!props || Object.keys(props.news).length === 0) return;
    await $db?.wait();
    if (!props.news.text_body || !props.news.html_body) {
      try {
        let ret: any = await $db.query(
          "select text_body, html_body from only $id",
          {
            id: props.news.id,
          },
        );
        props.news.text_body = ret[0].result.text_body;
        props.news.html_body = ret[0].result.html_body;
      } catch (error: any) {
        useToast().add({
          title: "Error querying news",
          description: error.toString(),
          timeout: 0,
        });
      }
    }
  },
  { deep: true, immediate: true },
);

let news: Ref<News> = ref({} as News);
let updatedNews = false;
watch(
  () => props.news,
  async () => {
    console.log("in watch, setting updatedNews to true");
    // updatedNews = true;
    Object.assign(news.value, props.news);
  },
  { deep: true },
);

async function updateNews(field?: keyof News) {
  console.log("in updatenews news", news);
  if (!news || Object.keys(news).length === 0 || !field) return;
  if (!news.value.rating || news.value.rating < 0 || news.value.rating > 100) {
    news.value.rating = 0;
  }
  console.log("in updatenews updatedNews", updatedNews);
  if (updatedNews) {
    updatedNews = false;
    return;
  }
  try {
    await $db?.wait();
    const update: Partial<News> = field ? { [field]: news.value[field] } : news;
    console.log(`merging ${news.value.id} with `, update);
    await $db?.merge<News>(news.value.id, update);
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
      {{ props.news.rating }}
      {{ news.rating }}
      <div class="flex flex-wrap">
        <UBadge class="m-1">
          <UInput
            variant="none"
            v-model.number="news.rating"
            placeholder="0"
            @vue:updated="updateNews('rating')"
            type="number"
            max="100"
            min="0"
            color="primary"
            class="w-[6.5rem]"
            :ui="{
              trailing: { padding: { sm: 'pe-12' } },
              padding: { sm: 'p-0' },
            }"
          >
            <template #trailing>rating</template>
          </UInput>
        </UBadge>
        <UBadge class="m-1">
          {{ new Date(news.date).toLocaleString() }}
        </UBadge>
        <UBadge class="m-1">provider: {{ news.provider }}</UBadge>
        <UBadge class="m-1">
          <span class="mr-2">has been used</span>
          <UToggle
            color="emerald"
            v-model="news.used"
            @vue:updated="updateNews('used')"
            class="ring"
          />
        </UBadge>
        <UBadge class="m-1">
          <a :href="news.link" target="_blank" rel="noopener noreferrer">
            {{ news.link }}
          </a>
        </UBadge>
        <UBadge class="m-1"> tags: {{ news.tags?.join(", ") }} </UBadge>
      </div>
    </template>
    <div>
      <UTooltip text="Title" class="w-full">
        <UTextarea
          class="w-full"
          color="primary"
          autoresize
          placeholder="Title..."
          v-model="news.title"
          :rows="1"
          @vue:updated="updateNews('title')"
        >
        </UTextarea>
      </UTooltip>
      <UTooltip text="Caption" class="w-full">
        <UTextarea
          class="mt-2 w-full"
          color="primary"
          autoresize
          placeholder="Caption..."
          v-model="news.caption"
          :rows="1"
          @vue:updated="updateNews('caption')"
        ></UTextarea>
      </UTooltip>
      <UTooltip text="Clean Text" class="w-full">
        <UTextarea
          class="mt-2 w-full"
          autoresize
          placeholder="Text..."
          v-model="news.text_body"
          color="primary"
          :rows="1"
          @vue:updated="updateNews('text_body')"
        >
        </UTextarea>
      </UTooltip>
      <div class="w-full" v-show="news.html_body?.length || 0">
        <h1 class="w-full text-center font-bold">original text:</h1>
        <span v-html="news.html_body"></span>
      </div>
    </div>
  </UCard>
</template>
