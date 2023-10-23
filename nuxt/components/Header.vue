<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
</script>
<template>
  <div>
    <div class="inline-flex w-full py-2 px-4">
      <h1 class="text-2xl self-start hidden sm:block">news-scraper</h1>
      <div class="flex-grow self-center text-center">
        <UBadge>
      </div>
      <div class="self-end">
        <UButton v-if="$dbhelper.authenticated.value">logout</UButton>
      </div>
    </div>
    <slot />
  </div>
</template>
