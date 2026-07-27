Metavar = new SelfInitializingType of TokenTree

metavarNode = method()
metavarNode String := Metavar => name -> Metavar(name, {}, null, null)
metavarNode (String, String) := Metavar => (name, kind) -> Metavar(name, {}, null, kind)

isMetavar = t -> instance(t, Metavar)
metavarName = t -> leftOf t
metavarKind = t -> delimiterOf t

Repetition = new SelfInitializingType of TokenTree

repetitionNode = (quantifier, sep, unit) -> Repetition(quantifier, unit, null, sep)
isRepetition = t -> instance(t, Repetition)
repQuantifier = t -> leftOf t
repSeparator = t -> delimiterOf t
repUnit = t -> contentOf t

Alternation = new SelfInitializingType of TokenTree

alternationNode = branches -> Alternation(null, branches, null, null)
isAlternation = t -> instance(t, Alternation)
altBranches = t -> contentOf t

isSeqNode = t -> not isRepetition t and
    (delimiterOf t === "," or delimiterOf t === ";" or delimiterOf t === statementSeparator)

nodeKind = t -> (
    if isComment t then "Comment"
    else if isMacroInvocation t then "MacroInvocation"
    else if isMetavar t then "Metavar"
    else if isRepetition t then "Repetition"
    else if isAlternation t then "Alternation"
    else if isLeaf t then (
        s := leftOf t;
        if s === null then "Node"
        else if s#0 == "\"" then "String"
        else if match("^[0-9]", s) then "Number"
        else if match("^[A-Za-z]", s) then (if m2Keywords#?s then "Keyword" else "Identifier")
        else "Operator")
    else (
        d := delimiterOf t;
        if d === spaceOperator then "Apply"
        else if d === statementSeparator then "Statements"
        else if d === whitespaceDelimiter then (
            cs := contentOf t;
            if #cs == 0 then "Clause" else capitalize leftOf cs#0)
        else if d === "," or d === ";" then "Sequence"
        else if d === "->" then "Arrow"
        else if instance(d, String) then "Infix"
        else if leftOf t =!= null and rightOf t =!= null then "Bracket"
        else if leftOf t =!= null then "Prefix"
        else if rightOf t =!= null then "Postfix"
        else "Node"))

nodeKindNames = set {"Comment", "MacroInvocation", "Metavar", "Repetition", "Alternation", "String",
    "Number", "Keyword", "Identifier", "Operator", "Apply", "Sequence", "Arrow",
    "Infix", "Bracket", "Prefix", "Postfix", "If", "While", "For", "Try", "New",
    "Statements", "Clause", "Node"}

metavarPlaceholderPrefix = "MetavarHolePlaceholder"
metavarKindPrefix = "MetavarKind"
toPlaceholders = src -> (
    typed := replace(///(?<![A-Za-z0-9'])'([A-Za-z][A-Za-z0-9]*):([A-Za-z][A-Za-z0-9]*)///,
        concatenate(metavarKindPrefix, "$2(", metavarPlaceholderPrefix, "$1)"), src);
    replace(///(?<![A-Za-z0-9'])'([A-Za-z][A-Za-z0-9]*)///, metavarPlaceholderPrefix | "$1", typed))

repCallNames = new HashTable from {"+" => "RepPlus", "*" => "RepStar", "|" => "Alt"}
isIdentChar = c -> match("[A-Za-z0-9']", c)
scanReps = src -> (
    n := #src;
    at := i -> if i >= 0 and i < n then substring(i, 1, src) else "";
    stack := {};
    spans := {};
    for i to n - 1 do (
        c := at i;
        if c == "{" then (
            isFormOpen := i >= 1 and at(i - 1) == "'" and (i < 2 or not isIdentChar at(i - 2));
            stack = append(stack, (i, isFormOpen)))
        else if c == "}" then (
            if #stack == 0 then error "scanReps: unbalanced }";
            top := last stack;
            stack = drop(stack, -1);
            if top#1 then (
                form := if at(i + 1) == "+" or at(i + 1) == "*" then at(i + 1) else "|";
                spans = append(spans, (top#0 - 1, i, form)))));
    if #spans == 0 then return src;
    opens := hashTable apply(spans, s -> (s#0, repCallNames#(s#2) | "("));
    closes := hashTable apply(spans, s -> (s#1, if s#2 === "|" then 1 else 2));
    out := "";
    j := 0;
    while j < n do (
        if opens#?j then (out |= opens#j;
            j += 2)
        else if closes#?j then (out |= ")";
            j += closes#j)
        else (out |= at j;
            j += 1));
    out)

quantifierOf = t -> (
    if delimiterOf t === spaceOperator and #contentOf t == 2 and isLeaf (contentOf t)#0
    then (n := leftOf (contentOf t)#0;
        if n === "RepPlus" then "+" else if n === "RepStar" then "*"))

isNullElement = t -> isLeaf t and leftOf t === "null"
unitOf = t -> (
    inner := (contentOf (contentOf t)#1)#0;
    sep := if isSeqNode inner then delimiterOf inner else ",";
    elems := if isSeqNode inner then contentOf inner else {inner};
    while #elems > 0 and isNullElement last elems do elems = drop(elems, -1);
    (sep, elems))

altCallName = "Alt"
altInnerOf = t -> (
    if delimiterOf t === spaceOperator and #contentOf t == 2 and isLeaf (contentOf t)#0
    and leftOf (contentOf t)#0 === altCallName
    then (inner := contentOf (contentOf t)#1;
        if #inner == 0 then error "empty '{ | } alternation";
        inner#0))

altBranchesOf = t -> (
    if delimiterOf t === "|" and #contentOf t == 2
    then join(altBranchesOf (contentOf t)#0, altBranchesOf (contentOf t)#1)
    else {t})

typedKindOf = t -> (
    if delimiterOf t === spaceOperator and #contentOf t == 2 and isLeaf (contentOf t)#0
    and match("^" | metavarKindPrefix, leftOf (contentOf t)#0)
    then substring(#metavarKindPrefix, leftOf (contentOf t)#0))

markNodes = t -> (
    if isLeaf t then (
        if leftOf t =!= null and match("^" | metavarPlaceholderPrefix, leftOf t)
        then metavarNode substring(#metavarPlaceholderPrefix, leftOf t) else t)
    else if typedKindOf t =!= null then (
        kind := typedKindOf t;
        if not nodeKindNames#?kind then error("unknown node kind in pattern: '" | kind);
        hole := leftOf (contentOf (contentOf t)#1)#0;
        metavarNode(substring(#metavarPlaceholderPrefix, hole), kind))
    else if quantifierOf t =!= null then (
        (sep, elems) := unitOf t;
        repetitionNode(quantifierOf t, sep, apply(elems, markNodes)))
    else if altInnerOf t =!= null then
        alternationNode apply(altBranchesOf altInnerOf t, markNodes)
    else (setContent(t, apply(contentOf t, markNodes));
        t))

templateCache = new CacheTable
parseTemplate = src ->
templateCache#src ??= markNodes parseMacroTree toPlaceholders scanReps src

metavarNamesIn = t -> (
    if isMetavar t then {metavarName t}
    else flatten apply(contentOf t, metavarNamesIn))

treeEquals = (a, b) -> (
    leftOf a === leftOf b and rightOf a === rightOf b and delimiterOf a === delimiterOf b
    and #contentOf a == #contentOf b
    and all(#contentOf a, i -> treeEquals((contentOf a)#i, (contentOf b)#i)))

matchRepetition = (rep, ielems, b) -> (
    unit := repUnit rep;
    u := #unit;
    if u == 0 then error "empty '{ } repetition unit";
    scan(metavarNamesIn rep, nm -> if b#?nm and not instance(b#nm, List) then
            error("metavariable '" | nm | " is bound both outside and inside a repetition"));
    if #ielems % u != 0 then return false;
    nChunks := #ielems // u;
    if repQuantifier rep === "+" and nChunks == 0 then return false;
    ok := all(nChunks, ci -> (
        tb := new MutableHashTable;
        chunkOK := all(u, j -> matchInto(unit#j, ielems#(ci * u + j), tb));
        if chunkOK then scan(keys tb, nm -> b#nm = append(b#nm ?? {}, tb#nm));
        chunkOK));
    if ok and nChunks == 0 then scan(metavarNamesIn rep, nm -> b#nm ??= {});
    ok)

matchElems = (pelems, ielems, b) -> (
    reps := positions(pelems, isRepetition);
    if #reps == 0 then #pelems == #ielems and all(#pelems, i -> matchInto(pelems#i, ielems#i, b))
    else if #reps > 1 then error "a pattern sequence may hold at most one '{ } repetition"
    else (
        r := first reps;
        before := take(pelems, r);
        after := drop(pelems, r + 1);
        if #ielems < #before + #after then return false;
        nRep := #ielems - #before - #after;
        all(#before, i -> matchInto(before#i, ielems#i, b))
        and all(#after, i -> matchInto(after#i, ielems#(#before + nRep + i), b))
        and matchRepetition(pelems#r, take(drop(ielems, #before), nRep), b)))

matchInto = (pat, inp, b) -> (
    if isMetavar pat then (
        if metavarKind pat =!= null and nodeKind inp =!= metavarKind pat then false
        else (
            name := metavarName pat;
            if b#?name then treeEquals(b#name, inp)
            else (b#name = inp;
                true))
    )
    else if isAlternation pat then (
        matched := false;
        for branch in altBranches pat when not matched do (
            tb := new MutableHashTable;
            if matchInto(branch, inp, tb) and all(keys tb, k -> not b#?k or treeEquals(b#k, tb#k))
            then (scan(keys tb, k -> b#k = tb#k);
                matched = true));
        matched
    )
    else if isRepetition pat then
        matchRepetition(pat, if isSeqNode inp then contentOf inp else {inp}, b)
    else if isSeqNode pat and any(contentOf pat, isRepetition) then (
        if isSeqNode inp and delimiterOf pat === delimiterOf inp then matchElems(contentOf pat,
            contentOf inp, b)
        else if isSeqNode inp then false
        else matchElems(contentOf pat, {inp}, b)
    )
    else if #contentOf pat == 1 and isRepetition first contentOf pat
    and leftOf pat === leftOf inp and rightOf pat === rightOf inp
    and delimiterOf pat === delimiterOf inp then
        matchRepetition(first contentOf pat,
        flatten apply(contentOf inp, ic -> if isSeqNode ic then contentOf ic else {ic}), b)
    else if leftOf pat =!= leftOf inp or rightOf pat =!= rightOf inp
    or delimiterOf pat =!= delimiterOf inp
    or #contentOf pat =!= #contentOf inp then false
    else (
        cs := contentOf pat;
        ds := contentOf inp;
        all(#cs, i -> matchInto(cs#i, ds#i, b))
    ))

matchPattern = (pat, inp) -> (
    b := new MutableHashTable;
    if matchInto(pat, inp, b) then new HashTable from b)

cloneTree = t -> (class t)(leftOf t, apply(contentOf t, cloneTree), rightOf t, delimiterOf t)

expandRepetition = (rep, b) -> (
    unit := repUnit rep;
    names := select(metavarNamesIn rep, nm -> b#?nm);
    lengths := unique apply(names, nm -> #(b#nm));
    if #lengths > 1 then error "template repetition metavariables have differing lengths";
    nReps := if #names == 0 then 0 else first lengths;
    flatten apply(nReps, i -> (
        perRep := hashTable apply(names, nm -> (nm, (b#nm)#i));
        apply(unit, u -> instantiate(u, perRep)))))

instantiate = (tmpl, b) -> (
    if isAlternation tmpl then
        error "alternation '{ a | b } is a pattern-only construct, not valid in a template";
    if isMetavar tmpl then (
        name := metavarName tmpl;
        if not b#?name
        then error("template metavariable '" | name | " is unbound");
        cloneTree b#name
    )
    else if any(contentOf tmpl, isRepetition) then (
        if isSeqNode tmpl then
            TokenTree(leftOf tmpl,
            flatten apply(contentOf tmpl, c -> if isRepetition c then expandRepetition(c, b) else {
                instantiate(c, b)}),
            rightOf tmpl, delimiterOf tmpl)
        else (
            if #contentOf tmpl != 1 then
                error "a repetition '{ }+ in a template must be the only content of a sequence or bracket";
            rep := first contentOf tmpl;
            inner := delimited(repSeparator rep, expandRepetition(rep, b));
            TokenTree(leftOf tmpl, {inner}, rightOf tmpl, delimiterOf tmpl))
    )
    else (class tmpl)(leftOf tmpl, apply(contentOf tmpl, c -> instantiate(c, b)), rightOf tmpl,
        delimiterOf tmpl))

quote = method(Dispatch => Thing)
quote String := TokenTree => src -> instantiate(parseTemplate src, new HashTable)
quote Sequence := TokenTree => s -> (
    rest := drop(s, 1);
    binding := if #rest == 1 and instance(first rest, HashTable) then first rest
        else hashTable apply(rest, o -> (toString o#0, o#1));
    instantiate(parseTemplate first s, binding))

matchesIn = method()
matchesIn (TokenTree, TokenTree) := List => (pat, tree) -> (
    below := flatten apply(contentOf tree, c -> matchesIn(pat, c));
    here := matchPattern(pat, tree);
    if here =!= null then prepend((tree, here), below) else below)
patternCell = src -> (
    p := parseTemplate src;
    if delimiterOf p === statementSeparator and #contentOf p == 1 then (contentOf p)#0 else p)
matchesIn (String, TokenTree) := List => (patSrc, tree) -> matchesIn(patternCell patSrc, tree)

expandRules = (name, rules, inp) -> (
    for r in rules do (
        (pat, tmpl) := r;
        b := matchPattern(pat, inp);
        if b =!= null then
            return instantiate(tmpl, b)
    );
    error(name | ": no rule matched the input"))

declMacro = method()

declMacro (String, List) := Macro => (name, rules) -> (
    scan(rules, r -> if not ((instance(r, Sequence) or instance(r, List)) and #r == 2) then
            error(name | ": each rule must be a (pattern, template) pair, got " | toString r));
    parsed := apply(rules, r -> (parseTemplate r#0, parseTemplate r#1));
    installMacro(name, ts -> expandRules(name, parsed, focus ts)))

declMacro (String, String, String) := Macro => (name, p, t) -> declMacro(name, {(p, t)})
