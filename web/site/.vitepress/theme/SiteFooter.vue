<script setup lang="ts">
import { useData } from 'vitepress'

const { page } = useData()
</script>

<template>
  <!-- The homepage has its own footer; the 404 page needs none. -->
  <footer v-if="page.relativePath !== 'index.md' && !page.isNotFound" class="site-foot">
    <a href="/privacy">Privacy</a>
  </footer>
</template>
