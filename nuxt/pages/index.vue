<script setup lang="ts">
const route = useRoute();
const db_ready = ref(false);
db_ready.value = (await $db.ready) || false;
</script>

<template>
  <div>db status: {{ db_ready }}</div>
</template>
