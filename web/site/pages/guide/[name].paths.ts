import { guideFiles, guideSource } from '../../.vitepress/guide'
import { installRepo } from '../../.vitepress/site'

export default {
  paths() {
    return guideFiles().map((page) => ({ params: { name: page.name }, content: guideSource(page, installRepo) }))
  },
}
