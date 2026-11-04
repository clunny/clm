<script setup lang="ts">
import { nextTick, onMounted } from 'vue'
import { useData } from 'vitepress'

const { page } = useData()

// VitePress titles its own 404 page "404" in the browser; the built file says "Page not found | CLM"
// (`page-metadata`), so the browser's title is set to match once the page is up.
onMounted(async () => {
  await nextTick()
  if (page.value.isNotFound) document.title = 'Page not found | CLM'
})
</script>

<template>
  <!-- The homepage has its own footer; the 404 page needs none. -->
  <footer v-if="page.relativePath !== 'index.md' && !page.isNotFound" class="site-foot">
    <a href="/privacy">Privacy</a>
  </footer>
</template>
