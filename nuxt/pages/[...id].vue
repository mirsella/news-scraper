<script setup lang="ts">
const { $db, dbhelper } = useNuxtApp();
function updateurl() {
  navigateTo(Math.random().toString());
}
function removeurl() {
  navigateTo("/");
}
</script>

<template>
  <div>
    <NewsModal />
    <UButton @click="updateurl">update url</UButton>
    <UButton @click="removeurl">remove url</UButton>
    here will be the table
  </div>
</template>
