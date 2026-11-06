# Document highlights

Put the cursor on a name and a language server highlights every occurrence of
it: `textDocument/documentHighlight`. Each occurrence is a *text*, *read* or
*write* highlight, and an editor draws them as a background. They are not
semantic tokens, but marks can show them all the same: `highlight` is the token,
its modifiers say what kind of occurrence it is, and the `cursor` modifier draws
the cursor on the first character of the token.

````markdown
```rust,lsp
let mut §highlight.write.cursor|total§ = 0;
§highlight.write|total§ += value;
```
````

Read occurrences are blue, write occurrences red and plain text occurrences
yellow, from the palette of whichever theme the book is shown in. Like every
other token, they can be restyled with CSS.

```rust,lsp,noplayground,pane="Cursor on a variable"
fn running_total(values: &[u32]) -> u32 {
    let mut §highlight.write.cursor|total§ = 0;
    for value in values {
        §highlight.write|total§ += value;
        if §highlight.read|total§ > 100 {
            return §highlight.read|total§;
        }
    }
    §highlight.read|total§
}
```

```rust,lsp,noplayground,pane="Cursor on a type"
struct §highlight.text.cursor|Job§ {
    ready: bool,
}

fn next(queue: &mut Vec<§highlight.text|Job§>) -> Option<§highlight.text|Job§> {
    let position = queue.iter().position(|job: &§highlight.text|Job§| job.ready)?;
    Some(queue.remove(position))
}
```

```rust,lsp,noplayground,pane="Cursor on a keyword"
let found = §highlight.text.cursor|loop§ {
    match queue.pop() {
        Some(job) if job.ready => §highlight.text|break§ Some(job),
        Some(job) => retry.push(job),
        None => §highlight.text|break§ None,
    }
};
```

On a keyword the occurrences are the places control flow leaves: with the cursor
on `loop`, rust-analyzer highlights each `break` that ends it.
