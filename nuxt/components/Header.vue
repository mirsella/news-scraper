<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();
setInterval(async () => {
  // console.log($db.status, await $db.ready);
  if ($db.status !== 0) $dbhelper.connected.value = false;
}, 1000);
</script>
<template>
  <div>
    <div class="inline-flex w-full py-2 md:py-4 px-4 h-auto">
      <h1 class="text-2xl self-start hidden sm:block">news-scraper</h1>
      <div class="flex-grow self-center text-center h-7">
        <ClientOnly>
          <UBadge
            class="text-sm md:text-xl mx-1"
            :color="$dbhelper?.connected.value ? 'green' : 'red'"
          >
            connection
            <UIcon
              v-if="$dbhelper?.connected.value"
              name="i-carbon-connection-signal"
              class="h-4 w-4 md:h-5 md:w-5 ml-1"
            />
            <UIcon
              v-else
              name="i-carbon-connection-signal-off"
              class="h-4 w-4 md:h-5 md:w-5 ml-1"
            />
          </UBadge>
          <UBadge
            class="text-sm md:text-xl mx-1"
            :color="$dbhelper?.authenticated.value ? 'green' : 'red'"
          >
            authentication
            <UIcon
              v-if="$dbhelper?.authenticated.value"
              name="i-carbon-rule-locked"
              class="h-4 w-4 md:h-5 md:w-5 ml-1"
            />
            <UIcon
              v-else
              name="i-carbon-rule-cancelled"
              class="h-4 w-4 md:h-5 md:w-5 ml-1"
            />
          </UBadge>
        </ClientOnly>
      </div>
      <div class="self-end">
        <UButton
          v-if="$dbhelper?.authenticated.value"
          class="text-sm md:text-xl -h-2"
          color="red"
        >
          signout<UIcon
            name="i-heroicons-arrow-small-right-20-solid"
            class="h-4 w-4 md:h-5 md:w-5"
          />
        </UButton>
      </div>
    </div>
    <slot />
  </div>
</template>
