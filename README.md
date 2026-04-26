# puml

A Rust CLI reimplementation of PlantUML that renders `.puml` / `.plantuml` sources to SVG natively — no Java, no
PlantUML JAR, no runtime dependencies.

## Goals

- 100% input compatibility with PlantUML — same source files work unchanged
- Semantically equivalent SVG output (not pixel-identical)
- A single self-contained binary

## Install

### Homebrew (macOS / Linux)

```bash
brew tap grahambrooks/puml https://github.com/grahambrooks/puml
brew install puml
```

### From source

```bash
cargo install --path .
```

### Prebuilt binaries

Grab a tarball from the [latest release](https://github.com/grahambrooks/puml/releases/latest) — x86_64 Linux, x86_64
macOS, and aarch64 macOS are published on every push to `main`.

## Usage

```bash
puml examples/sequence.puml -o out.svg
```

```
puml [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file (default: stdin)

Options:
  -o, --output <FILE>   Output file. Format inferred from extension —
                        `.png` rasterises via resvg, anything else writes SVG.
                        Default: SVG to stdout.
      --scale <N>       PNG scale factor (default 2 for retina sharpness)
  -t, --type <TYPE>     Force diagram type (sequence|class|activity|state|...)
      --theme <THEME>   Built-in theme name
  -w, --watch           Re-render on file change
  -v, --verbose         Show parse/layout debug info
  -h, --help
  -V, --version
```

```bash
puml examples/class.puml -o out.svg            # SVG (default)
puml examples/class.puml -o out.png            # PNG, 2x scale
puml examples/class.puml -o out.png --scale 1  # PNG, 1:1 pixels
puml examples/class.puml -o out.svg --watch    # re-render on save (Ctrl+C to stop)
```

Multiple diagrams in one file produce multiple output files (`out-1.svg`, `out-2.svg`, …).

`--watch` also tracks every file reached via `!include` and re-renders when any of them change. Parse errors land on stderr but don't kill the watcher — fix and save again.

### `init-genai` — set up AI authoring guidance

```bash
puml init-genai          # drop templates into the current directory
puml init-genai path/to/project
puml init-genai --force  # overwrite existing files
```

Drops a consistent set of "how to author `.puml` files in this project"
templates into the locations that downstream AI tools read:

| File                                     | Tool                          |
|------------------------------------------|-------------------------------|
| `CLAUDE.md`                              | Claude Code                   |
| `AGENTS.md`                              | Codex / Aider / OpenCode      |
| `.cursor/rules/puml.mdc`                 | Cursor                        |
| `.github/copilot-instructions.md`        | GitHub Copilot                |
| `.windsurfrules`                         | Windsurf                      |

Existing files are left untouched unless `--force` is passed; the summary
reports which files were created, skipped, or overwritten.

### Themes & dark mode

Shapes render as **outlines on a transparent body by default** (structurizr-style),
so the canvas background shows through and the same SVG reads naturally on
either a light or dark page. Differentiation between kinds comes from stroke
colour, stereotype labels, and shape geometry — not filled rectangles. Users
can still colour individual elements with `skinparam …BackgroundColor` or
`[#color]` tags on mindmap / activity nodes; those overrides are respected.

Every SVG `puml` emits **adapts to the viewer's OS theme by default** — light in
light viewers, dark in dark viewers, via a `@media (prefers-color-scheme: dark)`
rule inside the SVG itself. No opt-in, no separate light/dark files. Older
viewers that ignore the rule fall back to the light rendering.

Override with `--theme NAME` or `!theme NAME` inside the source:

```bash
puml examples/sequence.puml -o out.svg                  # adapts per viewer (default)
puml examples/sequence.puml --theme light -o out.svg    # static light palette
puml examples/sequence.puml --theme dark  -o out.svg    # static dark palette
```

| Preset   | Effect                                                                                                                                                                                                 |
|----------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `auto`   | **Default.** Base light palette plus a media query that flips the background + text colour when the viewer prefers dark. Shape fills (class blue, note yellow, choice amber) stay constant in both modes. |
| `light`  | Opt out of adaptation — bakes the classic light palette into the SVG. Use when you want the output to render identically regardless of viewer theme.                                                    |
| `dark`   | Near-black background, near-white text, muted cool shape fills. Static.                                                                                                                                |
| `plain`  | Monochrome class/sequence palette (white fill, dark borders). Static.                                                                                                                                  |
| `amiga`  | Retro deep-blue background with orange accents. Static.                                                                                                                                                |

## Examples

Each gallery entry below is rendered by `puml` itself from the source file linked above it. Regenerate with `make run`
or `for f in examples/*.puml; do puml "$f" -o "docs/examples/$(basename "$f" .puml).svg"; done`.

### Sequence — [examples/sequence.puml](examples/sequence.puml)

![Sequence diagram](docs/examples/sequence.svg)

### Class — [examples/class.puml](examples/class.puml)

![Class diagram](docs/examples/class.svg)

### Activity — [examples/activity.puml](examples/activity.puml)

![Activity diagram](docs/examples/activity.svg)

### State — [examples/state.puml](examples/state.puml)

![State diagram](docs/examples/state.svg)

### Use Case — [examples/usecase.puml](examples/usecase.puml)

![Use case diagram](docs/examples/usecase.svg)

### Timing — [examples/timing.puml](examples/timing.puml)

![Timing diagram](docs/examples/timing.svg)

### Mind Map — [examples/mindmap.puml](examples/mindmap.puml)

![Mind map](docs/examples/mindmap.svg)

### Component — [examples/component.puml](examples/component.puml)

![Component diagram](docs/examples/component.svg)

### Object — [examples/object.puml](examples/object.puml)

![Object diagram](docs/examples/object.svg)

### Deployment — [examples/deployment.puml](examples/deployment.puml)

![Deployment diagram](docs/examples/deployment.svg)

### C4 — [examples/c4-container.puml](examples/c4-container.puml)

![C4 container diagram](docs/examples/c4-container.svg)

## Diagram support

| Status   | Diagram                                                                         | Notes                                                                                                                                                                          |
|----------|---------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Shipping | Sequence, Class, Activity, State, Use Case, Timing, Mind Map, Gantt, Deployment | Full parser + layout + renderer with fixture coverage. Deployment uses real UML shapes (3D node, cloud outline, cylinders, folder tab, frame tab, folded document).            |
| Shipping | C4 (Container, Component, Context)                                              | `!include <C4/Container>` and friends translate `Person/System/Container/Component/Rel/Boundary` macros into puml constructs at preprocess time. Boundaries render flat (MVP). |
| Partial  | Component, Object                                                               | Render via the class pipeline. Missing: `[Foo]` bracket components, `attr = value` object members.                                                                             |
| Planned  | —                                                                               | All PlantUML diagram types currently parse and render.                                                                                                                         |

"Shipping" means: parses the common PlantUML idioms for that diagram, produces a readable SVG, has fixture + snapshot
coverage. It does **not** yet mean byte-level parity with `plantuml.jar`.

## Tech stack

- **Parser:** [`pest`](https://pest.rs/) (one PEG grammar per diagram type) with a [
  `logos`](https://github.com/maciejhirsz/logos) tokenizer pre-pass
- **SVG output:** the [`svg`](https://crates.io/crates/svg) crate (direct DOM construction)
- **Font metrics:** `rustybuzz` + `ttf-parser`
- **CLI:** `clap` v4 (derive API)
- **Testing:** [`insta`](https://insta.rs/) snapshot tests

## Architecture

```
.puml source
    └─► Preprocessor   (strip comments, resolve !include, expand !define)
        └─► Lexer      (logos tokens)
            └─► Parser (pest grammar → raw parse tree)
                └─► AST     (typed diagram nodes)
                    └─► Layout  (coordinates, sizes)
                        └─► SVG renderer (svg crate Document)
                            └─► stdout / file
```

Source layout mirrors the pipeline: `src/parser/`, `src/ast/`, `src/layout/`, `src/render/`, one module per diagram type
inside each.

## Development

```bash
make             # list available targets
make build       # fmt + clippy + cargo build
make test        # cargo test
make check       # fmt-check + clippy + test (pre-commit)
make snapshots   # update insta snapshots
make run         # render examples/sequence.puml to out.svg
```

After an intentional render change, review snapshot diffs:

```bash
cargo insta review
```

## Releases

Every push to `main` that touches source code runs the `Release` workflow, which:

1. Runs `fmt-check`, `clippy`, and `cargo test`.
2. Computes a date-based version (`YYYY.M.D`, with `-N` suffix for repeat releases on the same day).
3. Builds release tarballs for Linux x86_64, macOS x86_64, and macOS aarch64.
4. Publishes a GitHub release and updates `HomebrewFormula/puml.rb` in-place.

## Non-goals

- PNG / PDF / LaTeX output (SVG only)
- PlantUML server API compatibility
- Full preprocessor macro system (basic `!define` / `!if` only)
- Custom icon libraries
- Pixel-perfect rendering match to reference PlantUML

## License

MIT
