import { dateLabel, publishedPosts } from '../blog'

export interface PostCard {
  slug: string
  title: string
  summary: string
  dateLabel: string
  minutes: number
  tags: string[]
  part?: number
}

declare const data: PostCard[]
export { data }

// The published posts alone, newest first, for `/blog/` (`blog-index`): a draft never reaches
// a page's data.
export default {
  load(): PostCard[] {
    return publishedPosts().map((p) => ({
      slug: p.slug,
      title: p.title,
      summary: p.summary,
      dateLabel: dateLabel(p.date),
      minutes: p.minutes,
      tags: p.tags,
      part: p.part,
    }))
  },
}
