import { guideFiles, guideSource } from '../../.vitepress/guide'

export default {
  paths() {
    return guideFiles().map((page) => ({ params: { name: page.name }, content: guideSource(page) }))
  },
}
