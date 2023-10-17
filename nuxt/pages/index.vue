<script setup lang="ts">
const { $db } = useNuxtApp();
// setInterval(async () => {
//   // console.log(await $db.ready, $db.status, await $db.info());
//   let query = $db.select("news");
//   console.log(await query);
// }, 500);

// setTimeout(async () => {
//   await $db.connect("http://127.0.0.1:8000/rpc", {
//     ns: "news",
//     db: "news",
//     auth: {
//       user: "news",
//       pass: "arst",
//     },
//   });
// }, 2000);
</script>

<template>
  <!-- <div>db status: {{ $db.ready || "disconnected" }}</div> -->
</template>
