<script setup lang="ts">
const { $db, $dbhelper } = useNuxtApp();

async function signout() {
  $db.invalidate();
  $dbhelper.authenticated.value = false;
  window.localStorage.removeItem("jwt");
  navigateTo("/login");
}
</script>
<template>
  <div>
    <div class="inline-flex w-full pt-1 md:py-4 px-4 h-auto">
      <a
        class="text-2xl self-start cursor-pointer transition hover:scale-110"
        href="/"
      >
        <span class="hidden sm:block"> Gusnews </span>
        <span class="sm:hidden mt-2">
          <UIcon name="i-heroicons-home" />
        </span>
      </a>
      <div class="flex-grow self-center text-center">
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
        <ClientOnly>
          <UButton
            v-if="$dbhelper?.authenticated.value"
            class="text-sm md:text-xl py-1 transition hover:scale-110"
            color="red"
            @click="signout"
          >
            signout<UIcon
              name="i-heroicons-arrow-small-right-20-solid"
              class="h-4 w-4 md:h-5 md:w-5"
            />
          </UButton>
        </ClientOnly>
      </div>
    </div>
    <slot />
  </div>
</template>
