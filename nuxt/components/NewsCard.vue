<script setup lang="ts">
import type { News } from "~/utils/news";

const props = defineProps<{
  news: News;
}>();
</script>

<template>
  <div class="w-full h-full bg-red-400">{{ news }}</div>
</template>
