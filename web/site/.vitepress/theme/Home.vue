<script setup lang="ts">
import { useData } from 'vitepress'

const { theme } = useData()
const repo: string = theme.value.installRepo
const { headline, firstLine, githubUrl, githubStars } = theme.value as Record<string, string>
const cut = headline.indexOf('. ') + 1
</script>

<template>
  <div class="clm-home">
    <section class="hero">
      <div class="hero-text">
        <h1>
          {{ headline.slice(0, cut) }}<br />
          <em>{{ headline.slice(cut + 1) }}</em>
        </h1>
        <p>{{ firstLine }}</p>
        <div class="ctas">
          <a class="btn p" href="/guide/quickstart">Get started</a>
          <a class="btn s" href="#how-it-works">How it works ↓</a>
          <a class="btn g" :href="githubUrl" :aria-label="githubStars ? `GitHub, ${githubStars} stars` : 'GitHub'"
            ><svg class="gh-icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z" /></svg
            ><span>{{ githubStars || 'GitHub' }}</span></a
          >
        </div>
        <div class="cmd">
          <code>/plugin marketplace add {{ repo }}</code> · <code>/plugin install clm@clm</code> ·
          <code>/clm:init</code>
        </div>
      </div>
      <div class="ring">
        <div class="circle"></div>
        <div class="core">
          <div><b>Your intent</b>specs in git</div>
        </div>
        <div class="st t"><b>Declare</b><small>you approve, yes or no</small></div>
        <div class="st r"><b>Build</b><small>AI, test-first</small></div>
        <div class="st bt"><b>Prove</b><small>a test cites every rule</small></div>
        <div class="st l"><b>Reconcile</b><small>drift is rebuilt</small></div>
      </div>
    </section>

    <div id="how-it-works">
      <div class="sec">
        <div class="k">The split</div>
        <h2>You own the what. AI owns the how.</h2>
        <p>
          You and your team decide the architecture and the capabilities, with AI helping you think
          them through. Everything below that line is the AI's: designs, code, tests. You review
          intent, never diffs.
        </p>
        <div class="grid2">
          <div class="tile you">
            <b>You</b>Architecture: what must always hold.<br />Capabilities: what people can do
            with it.<br />One yes or no on each change that matters.
          </div>
          <div class="tile ai">
            <b>AI</b>Turns intent into a graph of specs.<br />Builds every node, test-first, in
            parallel.<br />Fixes what drifts, defers what's big.
          </div>
        </div>
      </div>

      <div class="sec">
        <div class="k">The loop</div>
        <h2>Reconciliation, borrowed from Kubernetes</h2>
        <p>
          CLM isn't a Kubernetes tool. It borrows one idea from it: declare the state you want, and
          let a controller close the gap, again and again.
        </p>
        <div class="grid2">
          <div class="tile k8">
            <b>Kubernetes</b>You declare the cluster you want. A controller compares it with what's
            running and acts until they match.
          </div>
          <div class="tile clm">
            <b>CLM</b>You declare the system you want. CLM compares it with the code, by content
            hash, per requirement, and AI builds until every requirement is proven again.
          </div>
        </div>
      </div>

      <div class="sec">
        <div class="k">Not a vibe-coded mess</div>
        <h2>Still maintainable a year from now</h2>
        <div class="grid3">
          <div class="tile">
            <b>Every rule has a test</b>A requirement counts as built only when a passing test cites
            it at its current version.
          </div>
          <div class="tile">
            <b>Drift can't land</b>Change a requirement and the code behind it is stale until it's
            rebuilt. Programs check it, not prose.
          </div>
          <div class="tile">
            <b>Nothing dead</b>Code no requirement asks for is removed. The spec describes the
            present; git keeps the past.
          </div>
        </div>
      </div>

      <div class="sec">
        <div class="k">Solo or as a team</div>
        <h2>Git is the only channel</h2>
        <p>
          Specs, claims, approvals and lands are files and branches in your repository. Many
          sessions, or many people, build at once, each on its own node, each landing through the
          same checks. No server to run, nothing to keep in sync.
        </p>
      </div>

      <div class="sec">
        <div class="k">Standing on shoulders</div>
        <h2>Where the ideas come from</h2>
        <div class="grid3">
          <div class="tile small">
            <b
              ><a href="https://arxiv.org/html/2606.27045v2">The Spec Growth Engine</a></b
            >The spec graph: per-node contracts, spine-scoped context, blocking drift checks.
          </div>
          <div class="tile small">
            <b
              ><a href="https://kubernetes.io/docs/concepts/architecture/controller/"
                >Kubernetes's controllers</a
              ></b
            >Reconciliation: declare the state you want, and let a controller close the gap.
          </div>
          <div class="tile small">
            <b><a href="https://github.com/obra/superpowers">superpowers</a></b>The craft:
            brainstorming before writing, test-driven builds, verification before done.
          </div>
          <div class="tile small">
            <b
              ><a href="https://claude.com/blog/the-ai-native-sdlc-playbook"
                >Anthropic's AI-native SDLC</a
              ></b
            >The workflow: intent written down first, humans review intent, AI implements.
          </div>
        </div>
      </div>
    </div>

    <section class="final">
      <h2>Start with one conversation</h2>
      <p>In Claude Code, on a new project or an existing one:</p>
      <pre>/plugin marketplace add {{ repo }}
/plugin install clm@clm
/clm:init</pre>
      <a class="btn p" href="/guide/quickstart">Read the quickstart</a>
    </section>
    <footer class="foot">
      <span>CLM is built with CLM.</span>
      <span><a href="/guide/">Guide</a> · <a href="/app/">Sign in</a> · <a href="/privacy">Privacy</a></span>
    </footer>
  </div>
</template>
