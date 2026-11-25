// The site's two names for itself, read by the config and by the pages' loaders alike.

// The repository the install commands and the GitHub links name: the one place it is written
// (`install-repository`).
export const installRepo = process.env.INSTALL_REPO || 'clunny/clm'

// The site's address: every absolute URL the build writes is made from it.
export const siteUrl = 'https://clm-app.dev'
