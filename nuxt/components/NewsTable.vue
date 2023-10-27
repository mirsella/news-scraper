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
  {
    label: "Id",
    key: "id",
    sortable: true,
  },
];
const toast = useToast();
function showTips() {
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
}
const props = defineProps<{ loading: boolean }>();
watch(
  () => props.loading,
  async (loading) => {
    console.log("showing tips");
    if (loading === false) showTips();
  },
);
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

// watch(PaginedNews, async (PaginedNews) => {
//   const els = document.getElementsByTagName("tr");
//   for (let el of els) {
//     console.log("adding event listener");
//     el.addEventListener("click", (event) => {
//       console.log("clicked", event);
//     });
//   }
// });

const columnsChoice = columns.map((c) => c.key);
const selectedColumns = ref<string[]>(["title", "rating", "note", "link"]);
if (process.client) {
  let localstorageColumns = window.localStorage.getItem("selectedColumns");
  if (localstorageColumns)
    selectedColumns.value = JSON.parse(localstorageColumns);

  let newsperpage = window.localStorage.getItem("NewsPerPage");
  if (newsperpage) pageCount.value = parseInt(newsperpage);

  watch(selectedColumns, async (v) =>
    window.localStorage.setItem("selectedColumns", JSON.stringify(v)),
  );
  watch(pageCount, async (v) =>
    window.localStorage.setItem("NewsPerPage", v.toString()),
  );
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
            <ClientOnly>
              <UPagination
                v-model="page"
                :page-count="pageCount"
                :total="FilteredNews.length"
              />
            </ClientOnly>
          </div>
        </UBadge>
      </div>
      <ClientOnly>
        <UTable
          :loading="loading"
          :rows="PaginedNews"
          :columns="FilteredColumns"
          class="w-full"
          :ui="{
            td: { base: 'max-w-[0] truncate !p-2' },
          }"
        >
          <template #used-data="{ row }">
            <UToggle v-model="row.used" @click="updateUsed(row)" />
          </template>
          <template #tags-data="{ row }">
            <div class="h-10 whitespace-normal truncate">
              {{ row.tags?.join(", ") }}
            </div>
          </template>
          <template #rating-data="{ row }">
            <div>{{ row.rating || "" }}</div>
          </template>
          <template #title-data="{ row }">
            <UTooltip
              :text="row.title"
              class="w-full whitespace-normal h-10 truncate"
            >
              <div>
                {{ row.title }}
              </div>
            </UTooltip>
          </template>
          <template #caption-data="{ row }">
            <UTooltip :text="row.caption">
              <div>{{ row.caption }}</div>
            </UTooltip>
          </template>
          <template #link-data="{ row }">
            <UTooltip :text="row.link">
              <a :href="row.link" target="_blank" rel="noopener noreferrer">{{
                row.link
              }}</a>
            </UTooltip>
          </template>
          <template #note-data="{ row }">
            <UTooltip
              :text="row.note"
              class="w-full h-10 whitespace-normal truncate"
            >
              <div>{{ row.note }}</div>
            </UTooltip>
          </template>
          <template #id-data="{ row }">
            <UTooltip :text="row.id">
              <div>{{ row.id.split(":")[1] }}</div>
            </UTooltip>
          </template>
          <template #date-data="{ row }">
            <UTooltip
              :text="new Date(row.date).toLocaleString('fr-FR')"
              class="w-full truncate whitespace-normal"
            >
              <div>{{ new Date(row.date).toLocaleString("fr-FR") }}</div>
            </UTooltip>
          </template>
        </UTable>
      </ClientOnly>
    </UCard>
  </div>
</template>
