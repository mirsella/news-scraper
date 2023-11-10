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

const newtag = ref("");
async function addTag() {
  newtag.value = newtag.value.trim();
  if (newtag.value && !news.value.tags?.includes(newtag.value)) {
    news.value.tags
      ? news.value.tags.push(newtag.value)
      : (news.value.tags = [newtag.value]);
    newtag.value = "";
    updateNews("tags");
  } else {
    newtag.value = "";
  }
}

let news: Ref<News> = ref({} as News);
let lastUpdate = new Date(0);
watch(
  () => props.news,
  async () => {
    // only update if the last update was more than 2 seconds ago, to not overwrite user data
    if (new Date().getTime() - lastUpdate.getTime() > 2000 || !news.value.id) {
      Object.assign(news.value, props.news);
      if (news.value.rating === -1) {
        news.value.rating = undefined;
      }
    }
  },
  { deep: true, immediate: true },
);

async function updateNews(field?: keyof News) {
  if (!news.value.id || !field) return;
  lastUpdate = new Date();
  // wait for the v-model to update...
  await new Promise((resolve) => setTimeout(resolve, 10));
  if (!news.value.rating || news.value.rating < 0 || news.value.rating > 100) {
    news.value.rating = 0;
  }
  try {
    const update: Partial<News> = field
      ? { [field]: news.value[field] }
      : news.value;
    console.log("update", update);
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
        <UBadge class="m-1">
          <span v-if="news.tags?.length">tags:</span>
          <span v-else>no tags</span>
          <div class="flex flex-wrap">
            <div
              v-for="tag of news.tags"
              class="m-[0.1rem] bg-green-500 rounded-lg px-1 group relative inline-block overflow-hidden"
              @click="
                news.tags = news.tags?.filter((t) => t !== tag);
                updateNews('tags');
              "
            >
              <p class="group-hover:blur-sm transition-all duration-200">
                {{ tag }}
              </p>
              <div
                class="opacity-0 group-hover:opacity-100 absolute top-0 left-0 text-center w-full transition-all duration-200"
              >
                <UIcon name="i-heroicons-trash" />
              </div>
            </div>
          </div>
          <UInput
            @keyup.enter="addTag"
            placeholder="add a tag"
            :ui="{
              padding: { sm: 'p-0 pl-1' },
              trailing: { padding: { sm: 'pe-4' } },
              icon: { trailing: { pointer: '', padding: { sm: 'pe-1' } } },
            }"
            class="w-[5.7rem] pl-1"
            v-model="newtag"
          >
            <template #trailing>
              <UIcon
                class="bg-primary p-0 !z-50 cursor-pointer"
                name="i-heroicons-plus"
                @click="addTag"
              />
            </template>
          </UInput>
        </UBadge>
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
