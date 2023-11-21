<template>
  <div class="text-center text-xl w-full">
    no news found for this id.
    <UButton
      label="go home."
      size="lg"
      class="m-2"
      @click="navigateTo({ path: '/', query: {} })"
    />
  </div>
</template>
