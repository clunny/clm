<script setup lang="ts">
import { useData } from 'vitepress'
import { data as posts } from './posts.data'

const { theme } = useData()
</script>

<template>
  <div class="clm-blog">
    <header class="blog-head">
      <div class="k">Blog</div>
      <h1>Building software when AI writes every line</h1>
      <p>{{ theme.blogDescription }}</p>
      <a class="rss" href="/blog/feed.xml" target="_self">RSS feed →</a>
    </header>
    <div v-if="posts.length > 0" class="blog-grid">
      <a v-for="p in posts" :key="p.slug" class="blog-card" :href="`/blog/${p.slug}`">
        <div class="meta">
          <span v-if="p.part" class="part">Part {{ p.part }}</span>
          <span>{{ p.dateLabel }}</span>
          <span>·</span>
          <span>{{ p.minutes }} min read</span>
        </div>
        <h3>{{ p.title }}</h3>
        <p>{{ p.summary }}</p>
        <div class="tags">
          <span v-for="t in p.tags" :key="t" class="tag">{{ t }}</span>
        </div>
      </a>
    </div>
    <p v-else class="blog-empty">Posts are coming.</p>
  </div>
</template>
