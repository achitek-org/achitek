# kopye

A fast and flexible project scaffolding tool written in Rust. kopye allows you to quickly bootstrap new projects from templates stored in git repositories or local directories.

> **kopye** (pronounced "ko-pyeh") is Haitian Creole for "to copy"

## Features

- **Multiple source types**: Clone templates from GitHub, GitLab, or any git repository, or use local directories
- **Interactive prompts**: Define variables in your templates and prompt users for values during scaffolding
- **Tera templating**: Powerful templating engine for dynamic file generation
- **Preview before apply**: Review the file structure before creating files
- **Transaction-based**: Safe file operations with rollback support
- **Blueprint organization**: Define multiple templates in a single repository

## Installation

### From source

```bash
git clone https://github.com/yourusername/kopye.git
cd kopye
cargo build --release
```

The binary will be available at `target/release/kopye`.

## Usage

### Copy a template

Copy a template from a repository to a destination directory:

```bash
kopye copy <repo> <template> <destination>
```

**Examples:**

```bash
# From GitHub
kopye copy gh:username/templates rust_bin my-project

# From GitLab
kopye copy gl:username/templates node my-node-app

# From SSH URL
kopye copy git@github.com:username/templates.git rust_bin my-project

# From HTTPS URL
kopye copy git+https://github.com/username/templates.git node my-project

# From local directory
kopye copy /path/to/templates rust_bin my-project
```

### List and select templates interactively

Browse available templates and select one interactively:

```bash
kopye list <repo>
```

This will:
1. Show all available templates from the repository
2. Prompt you to select a template
3. Ask for a destination directory
4. Prompt for any template variables
5. Show a preview of files to be created
6. Apply the changes after confirmation

### Enable verbose output

Use the `-v` or `--verbose` flag to see detailed logging:

```bash
kopye -v copy gh:username/templates rust_bin my-project
```

## Using kopye Programmatically

In addition to the CLI, kopye can be used as a library in your Rust projects.

### Add as a dependency

```toml
[dependencies]
kopye = { path = "../kopye" }
```

### API Usage

kopye provides two main API functions:

#### `copy_template`

Copy a template non-interactively:

```rust
use kopye::api::copy_template;

fn main() -> Result<(), kopye::api::KopyeError> {
    // Copy from GitHub
    copy_template("gh:username/templates", "rust_bin", "./my-project")?;

    // Copy from local directory
    copy_template("/path/to/templates", "node", "./my-app")?;

    Ok(())
}
```

#### `list_templates`

Interactive template selection and copying:

```rust
use kopye::api::list_templates;

fn main() -> Result<(), kopye::api::KopyeError> {
    // Prompts user to select template and destination
    list_templates("gh:username/templates")?;

    Ok(())
}
```

### Error Handling

The API uses `KopyeError` which provides detailed diagnostic information:

```rust
use kopye::api::{copy_template, KopyeError};

match copy_template("gh:user/templates", "rust_bin", "./project") {
    Ok(_) => println!("Template copied successfully!"),
    Err(KopyeError::Source(e)) => eprintln!("Source error: {}", e),
    Err(KopyeError::Template(e)) => eprintln!("Template error: {}", e),
    Err(KopyeError::Prompt(e)) => eprintln!("Prompt error: {}", e),
}
```

## Creating Templates

### Blueprint configuration

Templates are organized using a `blueprints.toml` file in the root of your template repository:

```toml
[rust_bin]
path = "./rust"

[node]
path = "./node"

[python]
path = "./python"
```

### Template structure

Each template directory can contain:

1. **Template files** with `.tera` extension for dynamic content
2. **Static files** that are copied as-is
3. **A `kopye.toml` file** defining prompts for variables

### Variable prompts

Define variables in `kopye.toml` within your template directory:

```toml
[project]
type = "string"
help = "Name of project"

[binary]
type = "bool"
help = "Is project a binary"

[target]
type = "string"
help = "Compilation targets"
choices = [
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "aarch64-unknown-linux-gnu",
]
multiselect = true
```

**Supported variable types:**
- `string`: Text input
- `bool`: Yes/no confirmation
- `string` with `choices`: Single or multiple selection

### Using variables in templates

Use Tera syntax in `.tera` files:

```rust
// main.rs.tera
fn main() {
    println!("Hello from {{ project }}!");
    {% if binary %}
    // Binary-specific code
    {% endif %}
}
```

Template file names can also use variables:

```
{{ project }}.rs.tera  →  my-project.rs
```

## Project Structure

kopye is organized as a Cargo workspace with three crates:

- **kopye**: Main CLI application
- **kopye_utils**: Shared utilities for VFS, transactions, and file operations
- **tampopo**: Dependency graph resolution for prompt ordering

## Repository Format Support

kopye supports various repository reference formats:

| Format | Example | Description |
|--------|---------|-------------|
| GitHub shorthand | `gh:account/repo` | GitHub repository |
| GitLab shorthand | `gl:account/repo` | GitLab repository |
| SSH URL | `git@host:account/repo.git` | Standard git SSH URL |
| HTTPS URL | `git+https://example.com/repo.git` | HTTPS git URL |
| Local path | `/path/to/templates` | Local directory |

## Development

### Build

```bash
cargo build
```

### Run tests

```bash
cargo test
```

### Run with logging

```bash
RUST_LOG=debug cargo run -- -v copy gh:user/templates template-name dest
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
