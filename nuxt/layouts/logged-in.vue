<template>
  <div>
    <div class="inline-flex w-full py-2 px-4">
      <h1 class="text-2xl self-start">news-scraper</h1>
      <div class="flex-grow self-center text-center">stats here</div>
      <div class="self-end">
        <UButton>logout</UButton>
      </div>
    </div>
    <slot />
  </div>
</template>
