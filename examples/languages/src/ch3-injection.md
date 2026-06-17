# Injection

The Markdown grammar's injections query highlights each fenced block inside a
Markdown block with the grammar named by its info string. `lua` is configured,
so it is sub-highlighted; `c` is **not** configured, so it is left as plain text
— an unregistered injected language degrades gracefully instead of erroring.

(The outer block uses four backticks so the inner three-backtick fences are
content, not terminators.)

````markdown
# A short document

```lua
local greeting = "hello"
print(greeting)
```

```c
int main(void) { return 0; }
```
````
