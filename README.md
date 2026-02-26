# mdbook-buildtime-annotations

## Configuring mdbook

Add the following to your `book.toml` and  season to taste.

```toml
[preprocessor.build-annotations]
command = "cargo run --bin mdbook-buildtime-annotations"
# the first n characters of the git commit hash to include in the annotation
commit_characters = 10
# the directory to look for a Cargo.toml in, relative to the book.
# defaults to "../" which is the parent directory of the book
# workspace_dir = "../"

# Defaults to true if unset, and determines whether to include the package name in the annotation
# package_name = true
# Defaults to true if unset, and determines whether to include the package version in the annotation
# package_version = true
# Defaults to true if unset, and determines whether to include the git commit in the annotation
# git_commit = true
```

It's well worth customising the footer tag by adding your own CSS, you can inject it by adding this to your config file:

```toml
[output.html]
additional-css = ["footer.css"]
```

And creating a CSS file targeting the specific tag we inject:

```css
footer#buildtime-annotations {
 color: #333;
 font-size: 0.8rem;
 display: flex;
 flex-direction: column;
 align-items: flex-end;
 margin-top: 1rem;
}
```

## Installation

```shell
cargo install mdbook-repo-annotations
```
