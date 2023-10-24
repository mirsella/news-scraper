<script setup lang="ts">
const { $db, dbhelper } = useNuxtApp();
function updateurl() {
  navigateTo(Math.random().toString());
}
</script>

<template>
  <div>
    <NewsModal />
    <UButton @click="updateurl">update url</UButton>
    here will be the table
  </div>
</template>
