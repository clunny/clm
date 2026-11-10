<script setup lang="ts">
import { computed } from 'vue'
import { useData } from 'vitepress'

interface Ref {
  slug: string
  title: string
  part?: number
}

// The route's params, made by `pages/blog/[slug].paths.ts` from the published posts alone.
const { params } = useData()
const post = computed(() => params.value as {
  slug: string
  title: string
  summary: string
  date: string
  dateLabel: string
  updated: string | null
  updatedLabel: string | null
  minutes: number
  tags: string[]
  part: number | null
  series: Required<Ref>[]
  prev: Ref | null
  next: Ref | null
})
</script>

<template>
  <article class="clm-blog clm-post">
    <div class="post-grid">
      <div class="post-main">
        <div class="crumb"><a href="/blog/">Blog</a><template v-if="post.part"> / Part {{ post.part }}</template></div>
        <h1>{{ post.title }}</h1>
        <p class="lede">{{ post.summary }}</p>
        <div class="byline">
          <time :datetime="post.date">{{ post.dateLabel }}</time>
          <template v-if="post.updated">
            <span>·</span>
            <span>Updated <time :datetime="post.updated">{{ post.updatedLabel }}</time></span>
          </template>
          <span>·</span>
          <span>{{ post.minutes }} min read</span>
          <template v-if="post.tags.length > 0">
            <span>·</span>
            <span>{{ post.tags.join(', ') }}</span>
          </template>
        </div>
        <div class="vp-doc post-body"><slot /></div>
      </div>
      <aside v-if="post.series.length > 0" class="post-series">
        <div class="box">
          <b>The series</b>
          <ol>
            <li v-for="s in post.series" :key="s.slug" :class="{ cur: s.slug === post.slug }">
              <template v-if="s.slug === post.slug">{{ s.title }}</template>
              <a v-else :href="`/blog/${s.slug}`">{{ s.title }}</a>
            </li>
          </ol>
        </div>
      </aside>
    </div>
    <nav v-if="post.prev || post.next" class="post-nav">
      <a v-if="post.prev" class="prev" :href="`/blog/${post.prev.slug}`">← Previous<b>{{ post.prev.title }}</b></a>
      <a v-if="post.next" class="next" :href="`/blog/${post.next.slug}`">Next →<b>{{ post.next.title }}</b></a>
    </nav>
  </article>
</template>
