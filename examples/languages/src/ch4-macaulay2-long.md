# Macaulay2 — a longer sample

A larger, realistic block exercising rings/types, juxtaposition calls, method
installation, control flow, and error handling — useful for eyeballing the
palette and the `language-m2` per-language overrides at scale.

```m2
Slice = new Type of BasicList


ZZ : ZZ := Slice => (i, j) -> (
    if i > j and (j >= 0 or i < 0) then
        error "slice [a:b] with a > b illegal";
    new Slice from {i, j}
)


ZZ : Slice := Slice => (n, s) -> (
    if s#1 == 0 then
        error "Slice: step cannot be zero";
    if #s != 2 then
        error ("expected 2 or 3 slice components, got " | toString(#s + 1));
    if n > s#0 and (s#0 >= 0 or n < 0) then
        error "slice [a:b] with a > b illegal";
    new Slice from {n, s#0, s#1}
)


String Array := (L, s) -> (
    if #s == 0 then
        error "expected at least one slice";
    l := last s;
    if instance(l, ZZ) then  (
        if l < 0 then l += #L;
        if l < 0 or l >= #L then
            error ("index " | toString(l) | " out of bounds for" | toString class L | " of length " | toString(#L));
        return L#l;
    );
    if not instance(l, Slice) then
        error "expected a slice or a number";
    (a, b, r) := (l#0, l#1, try l#2 else 1);
    if r < 0 then
        return reverse L[a:b:-r];
    if a < 0 then 
        a = a + #L;
    if b < 0 then 
        b = b + #L;
    if a < 0 or b < 0 then
        error ("index " | toString(min(a, b) - #L) " out of bounds for" | toString class L | " of length " | toString(#L));
    R := new class L from while a < b list L#a do a += r;
    if #s == 1 then 
        return R;
    R new Array from drop(toList s, -1)
);

filter = method();
filter (Function, BasicList) := BasicList => (f, L) -> (
    T := class L;
    new T from 
        for x in L list 
            if not (f x) then 
                continue;
    return x
)

map = method();
map (Function, BasicList) := BasicList => (f, L) -> 
    new class L from for x in L list f x
```
