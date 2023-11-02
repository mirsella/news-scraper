<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db } = useNuxtApp();

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
        let ret: any = await $db.query<[string, string]>(
          "select text_body, html_body from only $id",
          {
            id: props.news.id,
          },
        );
        news.value.text_body = ret[0].text_body;
        news.value.html_body = ret[0].html_body;
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
let lastUpdate = new Date(0);
watch(
  () => props.news,
  async () => {
    // only update if the last update was more than 2 seconds ago, to not overwrite user data
    if (new Date().getTime() - lastUpdate.getTime() > 2000 || !news.value.id) {
      Object.assign(news.value, props.news);
    }
  },
  { deep: true, immediate: true },
);

async function updateNews(field?: keyof News) {
  if (!news.value.id || !field) return;
  if (!news.value.rating || news.value.rating < 0 || news.value.rating > 100) {
    news.value.rating = 0;
  }
  lastUpdate = new Date();
  // wait for the v-model to update...
  await new Promise((resolve) => setTimeout(resolve, 10));
  try {
    const update: Partial<News> = field
      ? { [field]: news.value[field] }
      : news.value;
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
      <div class="flex flex-wrap">
        <UBadge class="m-1">
          <UInput
            variant="none"
            v-model.number="news.rating"
            placeholder="0"
            @input="updateNews('rating')"
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
            @click="updateNews('used')"
            v-model="news.used"
            class="ring"
          />
        </UBadge>
        <UBadge class="m-1">
          <a :href="news.link" target="_blank" rel="noopener noreferrer">
            {{ news.link }}
          </a>
        </UBadge>
        <UBadge class="m-1" v-if="news.tags?.length">
          tags: {{ news.tags?.join(", ") }}
        </UBadge>
        <UBadge class="m-1" v-else> no tags</UBadge>
      </div>
    </template>
    <div>
      <UTooltip text="Notes" class="w-full">
        <UTextarea
          class="w-full"
          color="primary"
          size="xl"
          autoresize
          placeholder="Notes..."
          v-model="news.note"
          :rows="1"
          @input="updateNews('note')"
        >
        </UTextarea>
      </UTooltip>
      <UTooltip text="Title" class="w-full">
        <UTextarea
          class="mt-2 w-full"
          size="xl"
          color="primary"
          autoresize
          placeholder="Title..."
          v-model="news.title"
          :rows="1"
          @input="updateNews('title')"
        >
        </UTextarea>
      </UTooltip>
      <UTooltip text="Caption" class="w-full">
        <UTextarea
          class="mt-2 w-full"
          color="primary"
          autoresize
          placeholder="Caption..."
          size="xl"
          v-model="news.caption"
          :rows="1"
          @input="updateNews('caption')"
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
          size="xl"
          @input="updateNews('text_body')"
        >
        </UTextarea>
      </UTooltip>
      <div class="w-full" v-show="news.html_body?.length || 0">
        <h1 class="w-full text-xl text-center font-bold">original text:</h1>
        <span v-html="news.html_body"></span>
      </div>
    </div>
  </UCard>
</template>
