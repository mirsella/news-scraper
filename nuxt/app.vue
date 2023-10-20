<script setup lang="ts">
const errors = useState<string[]>("errors", () => []);
errors.value.push("Hello world!");
errors.value.push("Hello arst!");
</script>

<template>
  <NuxtLayout>
    <ul class="p-2 w-full">
      <li v-for="error in errors" class="p-2">
        <UAlert
          icon="i-heroicons-x-circle-20-solid"
          color="red"
          title="Error"
          :description="error"
        />
      </li>
    </ul>
    <NuxtPage />
  </NuxtLayout>
</template>
