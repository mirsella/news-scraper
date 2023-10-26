<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db } = useNuxtApp();
const columns = [
  {
    label: "Title",
    key: "title",
    sortable: true,
  },
  {
    label: "Caption",
    key: "caption",
    sortable: true,
  },
  {
    label: "Rating",
    key: "rating",
    sortable: true,
    direction: "desc",
  },
  {
    label: "Tags",
    key: "tags",
    sortable: true,
  },
  {
    label: "Date",
    key: "date",
    sortable: true,
  },
  {
    label: "Note",
    key: "note",
    sortable: true,
  },
  {
    label: "Used",
    key: "used",
    sortable: true,
  },
  {
    label: "Link",
    key: "link",
    sortable: true,
  },
  {
    label: "Provider",
    key: "provider",
    sortable: true,
  },
];
const toast = useToast();
toast.add({
  id: "tips-sortcolumns",
  title: "Tips",
  description:
    "The columns sort options only apply to the current page. you can adjust how many news are shown per page.",
  icon: "i-carbon-information",
  color: "green",
  timeout: 7000,
});
toast.add({
  id: "tips-newsperpage",
  title: "Tips",
  description:
    "The more News per Page you have, the less responsive the page is.",
  icon: "i-carbon-information",
  color: "green",
  timeout: 7000,
});
defineProps<{ loading: boolean }>();
const news = useState<News[]>("news", () => []);

const page = ref(1);
const pageCount = ref(500);
const search = ref("");
const onlyNonused = ref(false);
const FilteredNews = computed(() =>
  news.value.filter((n) => {
    return (
      (n.title.toLowerCase().includes(search.value.toLowerCase()) ||
        n.caption.toLowerCase().includes(search.value.toLowerCase()) ||
        n.provider.toLowerCase().includes(search.value.toLowerCase()) ||
        n.note.toLowerCase().includes(search.value.toLowerCase())) &&
      (!onlyNonused.value || !n.used)
    );
  }),
);
const PaginedNews = computed(() =>
  FilteredNews.value.slice(
    (page.value - 1) * pageCount.value,
    page.value * pageCount.value,
  ),
);

const columnsChoice = columns.map((c) => c.key);
const selectedColumns = ref<string[]>(["title", "rating", "note", "link"]);
if (process.client) {
  let localstorageColumns = window.localStorage.getItem("selectedColumns");
  let newsperpage = window.localStorage.getItem("NewsPerPage");
  if (localstorageColumns) {
    selectedColumns.value = JSON.parse(localstorageColumns);
    // setTimeout(() => {
    //   selectedColumns.value = JSON.parse(localstorageColumns);
    // }, 1000);
  }
  if (newsperpage) {
    pageCount.value = parseInt(newsperpage);
    // setTimeout(() => {
    //   pageCount.value = JSON.parse(newsperpage);
    // }, 1000);
  }
  watch(selectedColumns, (v) => {
    window?.localStorage.setItem("selectedColumns", JSON.stringify(v));
  });
  watch(pageCount, (v) => {
    window?.localStorage.setItem("NewsPerPage", pageCount.value.toString());
  });
}

const FilteredColumns = computed(() => {
  if (selectedColumns.value.length === 0)
    selectedColumns.value = columns.map((c) => c.key);
  return columns.filter((c) =>
    selectedColumns.value.find((sc) => sc === c.key),
  );
});

async function updateUsed(row: News) {
  // need to inverse the value because the UI has not updated it yet
  const used = !row.used;
  const res = await $db.merge<News>(row.id, { used });
  if (!res[0]) {
    setTimeout(async () => {
      row.used = true;
      let n = news.value.find((n) => n.id === row.id) || ({} as any);
      n.used = false;
    }, 100);
    useToast().add({
      title: "Error",
      description:
        "Something went wrong while updating the News. maybe try to reconnect and refresh the page.",
      icon: "i-carbon-error",
      color: "red",
      timeout: 0,
    });
  }
}
</script>

<template>
  <div>
    <UCard
      class="w-full"
      :ui="{
        body: {
          padding: '',
          base: 'divide-y divide-gray-200 dark:divide-gray-700',
        },
      }"
    >
      <div class="flex flex-wrap gap-2 px-3 py-3">
        <UInput
          v-model="search"
          icon="i-heroicons-magnifying-glass-20-solid"
          placeholder="Search..."
          size="lg"
        />

        <UBadge color="gray">
          <UCheckbox v-model="onlyNonused" label="Non-used only" />
        </UBadge>

        <USelectMenu
          v-model="selectedColumns"
          :options="columnsChoice"
          multiple
        >
          <UButton icon="i-heroicons-view-columns" color="gray" size="lg">
            Columns
          </UButton>
        </USelectMenu>

        <UBadge color="gray">
          <div class="flex flex-wrap gap-2">
            <UInput
              v-model="pageCount"
              type="number"
              class="w-44"
              :ui="{ trailing: { padding: { sm: 'pe-24' } } }"
            >
              <template #trailing>
                <span class="text-gray-500 dark:text-gray-400">
                  News per Page
                </span>
              </template>
            </UInput>
            <UPagination
              v-model="page"
              :page-count="pageCount"
              :total="FilteredNews.length"
            />
          </div>
        </UBadge>
      </div>
      <ClientOnly>
        <UTable
          :loading="loading"
          :rows="PaginedNews"
          :columns="FilteredColumns"
          class="w-full"
          :ui="{ td: { base: 'max-w-[0] truncate' } }"
        >
          <template #used-data="{ row }">
            <UToggle v-model="row.used" @click="updateUsed(row)" />
          </template>
        </UTable>
      </ClientOnly>
    </UCard>
  </div>
</template>
