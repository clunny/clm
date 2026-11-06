import { loadPosts, postView, publishedPosts, resolvePostLinks } from '../../.vitepress/blog'

// One page for each published post; a draft has no page (`blog-drafts-left-out`).
export default {
  paths() {
    const all = loadPosts()
    const published = publishedPosts(all)
    return published.map((post) => ({
      params: postView(post, published),
      content: resolvePostLinks(post, all, { kind: 'page' }),
    }))
  },
}
