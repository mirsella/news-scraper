<script setup lang="ts">
const route = useRoute();
const isOpen = computed<boolean>({
  get: () => route.query.id !== undefined && route.query.id !== "",
  set: (value: boolean) => {
    if (!value) navigateTo({ query: {} });
  },
});
</script>

<template>
  <div>
    <UModal v-model="isOpen"> </UModal>
  </div>
</template>
