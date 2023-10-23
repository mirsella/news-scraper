<script setup lang="ts">
// const errors = useState<string[]>("errors", () => []);
</script>

<template>
  <Header />
  <!-- <ul class="p-2 pt-0 w-full" v-if="errors.length > 0"> -->
  <!--   <li v-for="error in errors" class="p-2"> -->
  <!--     <UAlert -->
  <!--       icon="i-heroicons-x-circle-20-solid" -->
  <!--       color="red" -->
  <!--       title="Error" -->
  <!--       :description="error" -->
  <!--     /> -->
  <!--   </li> -->
  <!-- </ul> -->
  <NuxtPage />
  <UNotifications />
</template>
