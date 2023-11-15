<script setup lang="ts">
import type { News } from "~/utils/news";
const { $db } = useNuxtApp();
const columns = [
  {
    key: "hiddenid",
  },
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
let props = defineProps<{ loading: boolean }>();
watch(
  () => props.loading,
  async (loading) => {
    if (loading === false) showTips();
  },
);
const news = useState<News[]>("news", () => []);

const page = ref(1);
const pageCount = ref(100);
watch(pageCount, async () => {
  if (pageCount.value > filteredNews.value.length) {
    page.value = 1;
  } else {
    page.value = Math.ceil(filteredNews.value.length / pageCount.value);
  }
});
const search = ref("");
const onlyNonused = ref(false);
const filteredNews = computed(() =>
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
const paginedNews = computed(() =>
  filteredNews.value.slice(
    (page.value - 1) * pageCount.value,
    page.value * pageCount.value,
  ),
);
if (process.client) {
  watch(
    paginedNews,
    (paginedNews) => {
      setTimeout(async () => {
        const els = document.querySelectorAll("tbody > tr");
        console.log("registering click events", els.length);
        for (let el of els) {
          el.addEventListener("click", (event) => {
            const parent = el;
            if (event.target?.localName === "td") {
              const id = parent.children[0].textContent;
              if (id) navigateTo({ query: { id } });
            }
          });
        }
      }, 100);
    },
    { immediate: true },
  );
}

const columnsChoice = columns
  .filter((c) => c.key != "hiddenid")
  .map((c) => c.key);
const selectedColumns = ref<string[]>(["title", "rating", "tags", "link"]);
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

const filteredColumns = computed(() => {
  if (selectedColumns.value.length === 0)
    selectedColumns.value = columns.map((c) => c.key);
  return columns.filter(
    (c) =>
      selectedColumns.value.find((sc) => sc === c.key) || c.key === "hiddenid",
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
              v-model.number="pageCount"
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
                :total="filteredNews.length"
              />
            </ClientOnly>
          </div>
        </UBadge>
      </div>
      <ClientOnly>
        <UTable
          :loading="loading"
          :rows="paginedNews"
          :columns="filteredColumns"
          class="w-full"
          :ui="{
            td: { base: 'max-w-[0] !p-2' },
          }"
        >
          <template #hiddenid-data="{ row }">
            {{ row.id }}
          </template>
          <template #used-data="{ row }">
            <UToggle v-model="row.used" @click="updateUsed(row)" />
          </template>
          <template #tags-data="{ row }">
            <UTooltip :text="row.tags?.join(', ')">
              <div class="h-10 whitespace-normal truncate max-w-full">
                {{ row.tags?.join(", ") }}
              </div>
            </UTooltip>
          </template>
          <template #rating-data="{ row }">
            <div class="w-max-full w-min">
              {{ row.rating != -1 ? row.rating : "" }}
            </div>
          </template>
          <template #title-data="{ row }">
            <UTooltip
              :text="row.title"
              class="whitespace-normal truncate h-10 max-w-full"
            >
              <div>
                {{ row.title }}
              </div>
            </UTooltip>
          </template>
          <template #caption-data="{ row }">
            <UTooltip :text="row.caption">
              <div class="whitespace-normal truncate h-10 max-w-full">
                {{ row.caption }}
              </div>
            </UTooltip>
          </template>
          <template #link-data="{ row }">
            <UTooltip :text="row.link" class="max-w-full">
              <a
                :href="row.link"
                target="_blank"
                rel="noopener noreferrer"
                class="whitespace-normal truncate max-h-10 max-w-full"
                >{{ row.link }}</a
              >
            </UTooltip>
          </template>
          <template #note-data="{ row }">
            <UTooltip :text="row.note">
              <div class="whitespace-normal truncate h-10 max-w-full">
                {{ row.note }}
              </div>
            </UTooltip>
          </template>
          <template #id-data="{ row }">
            <UTooltip :text="row.id">
              <div
                class="max-w-full whitespace-normal text-ellipsis break-all max-h-10"
              >
                {{ row.id.split(":")[1] }}
              </div>
            </UTooltip>
          </template>
          <template #date-data="{ row }">
            <UTooltip :text="new Date(row.date).toLocaleString()">
              <div class="max-w-full whitespace-normal text-ellipsis w-min">
                {{ new Date(row.date).toLocaleString() }}
              </div>
            </UTooltip>
          </template>
        </UTable>
      </ClientOnly>
    </UCard>
  </div>
</template>

<style>
/* hide the first column, which is the id of the row to handle the click */
th.text-left:nth-child(1) {
  display: none;
}
tbody.divide-y > tr > td:nth-child(1) {
  display: none;
}
</style>
