<script setup lang="ts">
import type { News } from "~/utils/news";
const news = useState<News[]>("news", () => []);
const props = defineProps<{ loading: boolean }>();
</script>

<template>
  <div>
    <UTable :loading="loading" :rows="news" />
  </div>
</template>
