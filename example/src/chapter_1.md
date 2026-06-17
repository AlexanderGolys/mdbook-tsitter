# Highlighting Macaulay2

A fenced block tagged `m2` (or `macaulay2`) is highlighted by the tree-sitter
grammar bundled into the preprocessor:

```m2
-- A polynomial ring and a monomial ideal
R = QQ[x,y,z];
I = ideal(x^2, x*y, y^3);
betti res I

needsPackage "Depth";
isCM = dim R - depth(I, R) == 0
```

An untagged block, or one in a language the preprocessor does not know, is left
for mdBook's default highlighter:

```text
this block is passed through untouched
```
