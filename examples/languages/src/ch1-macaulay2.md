# Macaulay2

A well-formed block highlights cleanly:

```m2
R = QQ[x,y,z];
I = ideal(x^2, x*y, y^3);
betti res I
```

A block with a syntax error still highlights what it can — tree-sitter recovers
from the error rather than failing the build:

```m2
f = method(
R = QQ[x
```
