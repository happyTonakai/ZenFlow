# ZenFlow

AI Paper & News Recommendation Agent — a personal RSS-based recommendation system that learns your preferences through feedback.

## Features

- **Smart Recommendations**: LLM-driven scoring learns your preferences from likes, clicks, and dislikes
- **arXiv Integration**: RSS feeds for any arXiv categories you follow
- **One-Click Paper Extraction**: Extract any arXiv paper as plain text
  - HTML-first: prefers the HTML version when available
  - TeX fallback: downloads, flattens, and converts LaTeX source
  - Multi-file support: automatically merges `\input`/`\include` references
  - Full LaTeX-to-text: macros, accents, math symbols, environments
- **Community Votes**: HuggingFace Papers & AlphaXiv upvote integration
- **Translation**: Baidu Translate integration for Chinese abstracts

## Architecture

- **Desktop App**: Tauri v2 (Rust backend + React/TypeScript frontend)
- **Database**: SQLite (local, single file)
- **AI**: OpenAI-compatible LLM API for scoring and preferences

## Paper Extraction

The paper extraction feature is inspired by and grateful to these open-source projects:

- [arxiv2md](https://github.com/lukas-blecher/arxiv2md) — HTML to Markdown conversion logic
- [arxiv-to-prompt](https://github.com/AgnostiqHQ/arxiv-to-prompt) — LaTeX source flattening, comment removal, macro expansion
- [pylatexenc](https://github.com/phfaist/pylatexenc) — LaTeX to Unicode/plain text conversion

## Build & Run

```bash
# Development
just dev

# Build
just build

# Run tests
just test
```

## License

See [LICENSE](LICENSE) for details.
