<script setup lang="ts">
import type { News } from "~/utils/news";

const props = defineProps<{
  news: News;
}>();
const BadgeCss = "m-1 min-h-8";
</script>

<template>
  <UCard>
    <template #header>
      <div class="flex flex-wrap">
        <UBadge :class="BadgeCss">
          <UInput
            variant="none"
            v-model="news.rating"
            placeholder="rating"
            type="number"
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
          <span class="mr-1">has been used</span>
          <UToggle color="gray" v-model="news.used" />
        </UBadge>
        <UBadge :class="BadgeCss">
          <a :href="news.link" target="_blank" rel="noopener noreferrer">
            {{ news.link }}
          </a>
        </UBadge>
      </div>
    </template>
  </UCard>
</template>
