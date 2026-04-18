# puml

A Rust CLI reimplementation of PlantUML that generates SVG natively — no Java, no PlantUML JAR, no runtime dependencies.

## Goal

100% input compatibility with PlantUML. Same `.puml` / `.plantuml` files work unchanged. SVG output is semantically equivalent (not pixel-identical).

## Tech Stack

- **Language**: Rust (stable)
- **Parser**: `pest` (PEG grammar files per diagram type)
- **Lexer**: `logos` (fast tokenization pre-pass)
- **SVG output**: `svg` crate (direct SVG DOM construction)
- **CLI**: `clap` (v4, derive API)
- **Font metrics**: `rustybuzz` + `ttf-parser` (text layout/measurement)
- **Testing**: `insta` (snapshot tests against expected SVG output)

## Architecture

```
src/
├── main.rs              # CLI entry point
├── cli.rs               # clap argument definitions
├── parser/
│   ├── mod.rs           # public parse() -> DiagramAst
│   ├── preprocessor.rs  # @startuml/@enduml, !include, !define, skinparam
│   ├── lexer.rs         # logos-based tokenizer
│   └── grammars/        # .pest grammar files, one per diagram type
│       ├── sequence.pest
│       ├── class.pest
│       ├── activity.pest
│       ├── state.pest
│       └── ...
├── ast/
│   ├── mod.rs           # DiagramAst enum (one variant per diagram type)
│   ├── sequence.rs      # SequenceDiagram AST nodes
│   ├── class.rs
│   └── ...
├── layout/
│   ├── mod.rs           # Layout trait: fn layout(ast) -> LayoutTree
│   ├── sequence.rs      # Sequence layout engine
│   ├── class.rs         # Graphviz-inspired force/rank layout
│   └── ...
├── render/
│   ├── mod.rs           # Render trait: fn render(layout) -> Document (svg crate)
│   ├── sequence.rs
│   ├── class.rs
│   ├── theme.rs         # skinparam → style resolution
│   └── primitives.rs    # shared SVG helpers (arrow, box, text, etc.)
└── error.rs             # PumlError enum
```

### Data Flow

```
.puml source
    └─► Preprocessor   (strip comments, resolve !include, expand !define)
            └─► Lexer  (logos tokens)
                └─► Parser  (pest grammar → raw parse tree)
                    └─► AST  (typed diagram nodes)
                        └─► Layout engine  (coordinates, sizes)
                            └─► SVG renderer  (svg crate Document)
                                └─► stdout / file
```

## Diagram Type Priority

Implement in this order. Each phase should be shippable.

| Phase | Diagram Types | Notes |
|-------|--------------|-------|
| 1 | Sequence | Most used; linear layout, well-defined algorithm |
| 2 | Class | Rank-based layout (Sugiyama / DOT-style) |
| 3 | Activity | Flowchart layout |
| 4 | State | State machine graph layout |
| 5 | Component, Deployment | Similar layout to class |
| 6 | Use Case, Object | Simpler subsets |
| 7 | Timing, Mind Map, Gantt | Specialized layouts |

## Key Implementation Notes

### Parser Strategy
- One `.pest` grammar file per diagram type under `src/parser/grammars/`
- Preprocessor runs first (before parsing) to handle `!include`, `!define`, and strip comments
- Diagram type is auto-detected from syntax markers (`->` → sequence, `class` keyword → class, etc.) or explicit `@startuml(type)` hint

### Layout
- Sequence diagrams: fixed column/row grid — simple and deterministic
- Class/Component diagrams: implement a simplified Sugiyama layered layout (nodes ranked by dependency depth, edges routed between ranks)
- No external layout library dependency — implement what's needed for each diagram type

### SVG Output
- Use the `svg` crate to build the document tree
- All text is embedded as `<text>` elements (not path-encoded fonts)
- Arrows use `<marker>` definitions for arrowheads
- Styles applied via `<style>` block at SVG root (not inline per-element)
- Default theme matches PlantUML's default visual appearance

### Skinparam / Themes
- Parse `skinparam` blocks into a `Theme` struct
- `Theme` is passed to the renderer; defaults mirror PlantUML defaults
- Support: `backgroundColor`, `sequenceArrowThickness`, `classBorderColor`, `FontName`, `FontSize`, etc.

### CLI Interface
```
puml [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file (default: stdin)

Options:
  -o, --output <FILE>   Output SVG file (default: stdout)
  -t, --type <TYPE>     Force diagram type (sequence|class|activity|...)
      --theme <THEME>   Built-in theme name
  -w, --watch           Re-render on file change
  -v, --verbose         Show parse/layout debug info
  -h, --help
  -V, --version
```

## Development Commands

Prefer the `Makefile` targets over raw `cargo` invocations — they're the same commands CI runs, so "works for me" and "works in CI" stay aligned. Run `make` with no arguments to list available targets.

```bash
make build          # fmt + clippy + cargo build
make test           # cargo test
make check          # fmt-check + clippy + test (use this before declaring done)
make snapshots      # update insta snapshots (INSTA_UPDATE=always)
make run            # render examples/sequence.puml to out.svg
make release        # cargo build --release
make clean          # cargo clean + remove out.svg
```

After an intentional render change, review snapshot diffs:

```bash
cargo insta review
```

## Verifying a task is complete

**Before reporting any code change as complete, run `make check` and confirm it passes.** This runs `fmt-check`, `clippy -D warnings`, and the full test suite — the same gates the release workflow enforces. If `make check` fails, the task is not done.

- If `clippy` flags something, fix it rather than silencing it with `#[allow(...)]` unless the lint is genuinely wrong for this code.
- If tests fail because snapshots legitimately changed, run `make snapshots` and inspect the diff before committing. Do not update snapshots blindly.
- If `fmt-check` fails, run `make fmt` — never hand-format.
- For UI/render changes, also run `make run` and open `out.svg` to confirm the output is sensible; snapshot equality alone doesn't prove the rendering is correct.

Only after `make check` passes clean should you summarize the work as done.

## Testing Strategy

- `tests/snapshots/` — insta snapshot SVGs for each diagram type
- `tests/fixtures/` — `.puml` input files mirroring real PlantUML examples
- For each diagram type: parse-only tests, layout tests (check bounding boxes), render tests (snapshot)
- Compatibility test suite: run puml and reference plantuml on same input, diff SVG structure (not pixels)

## Compatibility Constraints

- Must accept any valid PlantUML file without error (unknown directives → warn, not fail)
- `@startuml` / `@enduml` markers are optional per PlantUML spec
- Multiple diagrams in one file → multiple output files (e.g. `out-1.svg`, `out-2.svg`)
- `!include` resolves relative to the source file's directory

## Non-Goals (for now)

- PNG/PDF/LaTeX output (SVG only)
- PlantUML server API compatibility
- Full preprocessor macro system (basic `!define` / `!if` only)
- Custom icon libraries (stdlib icons only)
- Pixel-perfect rendering match to reference PlantUML
