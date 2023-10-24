<script setup lang="ts">
const route = useRoute();
const isOpen = computed<boolean>({
  get: () => route.params.id !== "",
  set: (value: boolean) => {
    if (!value) navigateTo("/");
  },
});
</script>

<template>
  <div>
    <UModal v-model="isOpen">
      <UCard>
        <template #header>header</template>
        content
        <template #footer>footer</template>
      </UCard>
    </UModal>
  </div>
</template>
