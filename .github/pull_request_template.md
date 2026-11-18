## Description

<!-- Describe the goal of this pull request and the changes made. -->

## Related Issues

<!-- Closes #123, Fixes #456 -->
Fixes #

## Type of Change

<!-- Please check the options that apply. -->

- [ ] 🐛 Bug fix (non-breaking change fixing an issue)
- [ ] ✨ New feature (non-breaking change adding functionality or desktop parity)
- [ ] ⚡ Performance improvement
- [ ] 🎨 UI / CSS / Theme styling improvement
- [ ] 📱 Mobile / PWA optimization
- [ ] 📝 Documentation update
- [ ] 🔧 Tooling, CI/CD, or repository automation
- [ ] 🔄 Refactoring (code cleanup without functional changes)

## Desktop Anki Parity

<!-- For UI/scheduler features: how does this match desktop Anki behavior? -->
- [ ] Matches Desktop Anki 23/24 behavior and visual style (`anki-tokens.css`)
- [ ] Retains FSRS / SM-2 scheduler fidelity
- [ ] Updated `docs/PARITY.md` (if applicable)

## Testing Performed

<!-- How did you verify these changes? -->

- [ ] Rust server tests passing (`cargo test` in `server/`)
- [ ] Svelte 5 type check passing (`npm run check` in `web/`)
- [ ] Svelte 5 build passing (`npm run build` in `web/`)
- [ ] Playwright E2E tests passing (`npx playwright test` in `e2e/`)
- [ ] Manual desktop browser testing (Chrome / Firefox / Safari)
- [ ] Manual mobile testing (iOS Safari / Android Chrome / 390px viewport)

## Screenshots or Recordings

<!-- If applicable, add before and after screenshots / GIFs. -->

| Before | After |
| :---: | :---: |
| *(Image)* | *(Image)* |

## Checklist

- [ ] My code adheres to the style guidelines of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] My changes generate no new compiler warnings or TypeScript errors
- [ ] All new and existing tests pass locally
- [ ] I have verified that no proprietary, copyrighted, or unlicensed code has been included
- [ ] AGPL-3.0 attribution and notice requirements are satisfied
