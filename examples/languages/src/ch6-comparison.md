# Highlighting, side by side

Each pair contains the same substantial source file. The first column is
rendered by `mdbook-tsitter`; the second is left to mdBook with the
`notreesitter` annotation. The examples favor real language features over toy
one-liners while keeping identifiers and line lengths comfortable in two
columns.

<div class="highlight-comparison-key">
  <strong>tree-sitter</strong>
  <strong>mdBook</strong>
</div>

## Rust

Traits, generic bounds, `Arc` delegation, concurrent maps, locks, iterators,
closures, and `let`-`else` in a compact workspace symbol index.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
{{#include ./snippets/rust.rs}}
```

</div>

<div class="highlight-comparison-pane">

```rust,notreesitter
{{#include ./snippets/rust.rs}}
```

</div>

</div>

## Python

Data classes, protocols, generics, async iteration, task groups, comprehensions,
enums, and structural pattern matching in a small job supervisor.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```python
{{#include ./snippets/python.py}}
```

</div>

<div class="highlight-comparison-pane">

```python,notreesitter
{{#include ./snippets/python.py}}
```

</div>

</div>

## TypeScript

Discriminated unions, generic interfaces, private fields, async generators,
type narrowing, and `satisfies` in a typed paginated cache.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```typescript
{{#include ./snippets/typescript.ts}}
```

</div>

<div class="highlight-comparison-pane">

```typescript,notreesitter
{{#include ./snippets/typescript.ts}}
```

</div>

</div>

## JavaScript

Private fields, closures, destructuring, optional chaining, promises, and async
generators in an event bus consuming a paginated API.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```javascript
{{#include ./snippets/javascript.js}}
```

</div>

<div class="highlight-comparison-pane">

```javascript,notreesitter
{{#include ./snippets/javascript.js}}
```

</div>

</div>

## Go

Type constraints, generic interfaces, goroutines, channels, `select`, contexts,
and error handling in a concurrent mapping pipeline.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```go
{{#include ./snippets/go.go}}
```

</div>

<div class="highlight-comparison-pane">

```go,notreesitter
{{#include ./snippets/go.go}}
```

</div>

</div>

## Java

Sealed interfaces, records, validation, streams, guarded pattern matching,
switch expressions, and virtual threads in a typed event log.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```java
{{#include ./snippets/java.java}}
```

</div>

<div class="highlight-comparison-pane">

```java,notreesitter
{{#include ./snippets/java.java}}
```

</div>

</div>

## C

Enums, tagged unions, slices, function pointers, dynamic storage, designated
initializers, and explicit cleanup in a small event queue.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```c
{{#include ./snippets/c.c}}
```

</div>

<div class="highlight-comparison-pane">

```c,notreesitter
{{#include ./snippets/c.c}}
```

</div>

</div>

## C++

Concepts, ranges, views, variants, smart pointers, lambdas, and move semantics
in a C++20 table pipeline.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```cpp
{{#include ./snippets/cpp.cpp}}
```

</div>

<div class="highlight-comparison-pane">

```cpp,notreesitter
{{#include ./snippets/cpp.cpp}}
```

</div>

</div>

## Bash

Strict mode, arrays, associative maps, traps, process substitution, pattern
matching, parameter expansion, and careful quoting in an artifact verifier.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```bash
{{#include ./snippets/bash.sh}}
```

</div>

<div class="highlight-comparison-pane">

```bash,notreesitter
{{#include ./snippets/bash.sh}}
```

</div>

</div>

## PHP

Attributes, enums, readonly classes, interfaces, generators, named arguments,
arrow functions, and `match` in a report job.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```php
{{#include ./snippets/php.php}}
```

</div>

<div class="highlight-comparison-pane">

```php,notreesitter
{{#include ./snippets/php.php}}
```

</div>

</div>

## Lua

Metatables, method syntax, closures, coroutine-based iteration, higher-order
functions, table operations, and protected calls in a lazy stream.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```lua
{{#include ./snippets/lua.lua}}
```

</div>

<div class="highlight-comparison-pane">

```lua,notreesitter
{{#include ./snippets/lua.lua}}
```

</div>

</div>

## Haskell

Algebraic data types, type classes, constrained polymorphism, higher-order
folds, pattern matching, comprehensions, and concurrent traversal.

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```haskell
{{#include ./snippets/haskell.hs}}
```

</div>

<div class="highlight-comparison-pane">

```haskell,notreesitter
{{#include ./snippets/haskell.hs}}
```

</div>

</div>
