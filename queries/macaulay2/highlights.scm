; Comments
[
  (line_comment)
  (block_comment)
] @comment

; Literals
(integer_literal) @number

(float_literal) @number.float

(string_literal) @string

(escape_sequence) @character.special

(raw_string_escape) @character.special

(symbol) @variable

(_
  symbol: (resolved_symbol) @variable)

; Operators
(binary_expression
  operator: _ @operator)

(prefix_expression
  operator: _ @operator)

(postfix_expression
  operator: _ @operator)

(lambda_expression
  operator: _ @operator)

; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<|"
  "|>"
] @punctuation.bracket

[
  ","
  ";"
] @punctuation.delimiter



[
  "if"
  "else"
  "then"
  "when"
] @keyword.conditional

[
  "for"
  "while"
  "in"
  "from"
  "to"
] @keyword.repeat

[
  "return"
  "break"
  "continue"
] @keyword.return

[
  "try"
  "catch"
  "throw"
  "trap"
  "except"
  "shield"
] @keyword.exception

[
  "global"
  "local"
  "symbol"
  "threadVariable"
  "threadLocal"
] @keyword.modifier

[
  "time"
  "timing"
  "elapsedTime"
  "elapsedTiming"
  "profile"
  "TEST"
  "breakpoint"
  "step"
] @keyword.debug

[
  "and"
  "or"
  "xor"
  "not"
] @keyword.operator

[
  "new"
  "of"
  "list"
  "do"
] @keyword

((binary_expression
  left: [
    (integer_literal)
    (float_literal)
  ]
  operator: "_"
  right: (symbol)) @number
  (#set! priority 151))

; Function parameters
(lambda_expression
  parameters: [
    (symbol) @variable.parameter
    (sequence
      (symbol) @variable.parameter)
    (list
      (symbol) @variable.parameter)
  ])

; Function definitions
(binary_expression
  left: (symbol) @function
  operator: [":=" "="]
  right: (lambda_expression))

(binary_expression
  left: (symbol) @function
  operator: [":=" "="]
  right: (binary_expression
    right: (lambda_expression)))

; Members, options, and properties
(binary_expression
  operator: ["." ".?" "#" "#?" "_"]
  right: (symbol) @property)

(binary_expression
  operator: "_" @property
  right: (integer_literal) @property)

; Types
(new_statement
  type: (_) @type
  (of_clause
    (_) @type))

(new_statement
  (from_clause
    "from" @keyword))

; Method installations
((binary_expression
  left: (binary_expression
    left: (_) @type
    operator: _ @function
    right: (_) @type)
  operator: [
    "="
    ":="
  ] @keyword.operator)
  (#not-any-of? @function "." ".?" "#" "_"))

((binary_expression
  left: (binary_expression
    left: (symbol) @function
    operator: "SPACE"
    right: (symbol) @type)
  operator: [
    "="
    ":="
  ] @keyword.operator)
  (#match? @function "[a-z].*"))

; A _ B := (x, y) -> x*y
(binary_expression
  left: (binary_expression
    left: (_) @type
    operator: "_" @function
    right: (_) @type)
  operator: ["=" ":="] @keyword.operator
  right: (lambda_expression))

; f ZZ := g
(binary_expression
  left: (binary_expression
    left: (symbol) @function
    operator: "SPACE"
    right: (sequence
      "(" @type
      [
        (symbol) @type
        "," @type
      ]*
      ")" @type))
  operator: [
    "="
    ":="
  ] @keyword.operator)

; - ZZ := x -> -x
(binary_expression
  left: (prefix_expression
    operator: _ @constructor
    operand: (symbol) @type)
  operator: [
    "="
    ":="
  ] @keyword.operator)

; ZZ ! := n -> if n > 0 then n*(n-1)! else 1
(binary_expression
  left: (postfix_expression
    operand: (symbol) @type
    operator: _ @constructor)
  operator: [
    "="
    ":="
  ] @keyword.operator)

((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: (sequence
    [
      (cobinding)
      (local_cobinding)
      (global_cobinding)
      (thread_cobinding)
    ]
    .
    (_) @type
    .
    (_) @type))
  (#eq? @function.builtin "installAssignmentMethod"))

(binary_expression
  left: (new_statement
    "new" @keyword
    (of_clause
      "of"? @keyword)?
    (from_clause
      "from" @keyword
      [
        (symbol) @type
        (sequence
          (symbol) @type)
      ])?)
  operator: ":=" @keyword.operator)

(new_statement
  type: _ @type)

; Builtins
((symbol) @variable.builtin
  (#match? @variable.builtin "^((o[1-9][0-9]*)|oo|ooo|oooo)$"))

((symbol) @constant.builtin
  (#any-of? @constant.builtin "CatalanConstant" "EulerConstant" "ii" "pi" "null"))

((symbol) @boolean
  (#any-of? @boolean "true" "false"))


((symbol) @variable.builtin
  (#any-of? @variable.builtin
    "allowableThreads" "debugLevel" "defaultPrecision" "engineDebugLevel" "errorDepth" "gbTrace"
    "interpreterDepth" "lineNumber" "loadDepth" "maxAllowableThreads" "maxExponent" "minExponent"
    "numTBBThreads" "printingAccuracy" "printingLeadLimit" "printingPrecision" "printingTimeLimit"
    "printingTrailLimit" "version" "printWidth" "recursionLimit" "typicalValues"))

; Special strings
((string_literal) @string.special.url
  (#match? @string.special.url "^http[s]?://.*"))

((string_literal) @string.special.url
  (#match? @string.special.url "^www\\..*"))

((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: [
    (string_literal) @string.special.url
    (sequence
      .
      (string_literal) @string.special.url)
  ])
  (#any-of? @function.builtin "splitWWW" "getWWW" "urlEncode"))

((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: [
    (string_literal) @string.regexp
    (sequence
      .
      (string_literal) @string.regexp)
  ])
  (#any-of? @function.builtin "match" "regex" "select"))

((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: (sequence
    (string_literal) @string.regexp
    (string_literal) @string.special
    (_)))
  (#eq? @function.builtin "replace"))

((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: (sequence
    (string_literal) @string.regexp
    (_)+))
  (#eq? @function.builtin "separate"))

; Packages
((binary_expression
  left: (symbol) @function.builtin
  operator: "SPACE"
  right: [
    (symbol) @module.builtin
    (sequence
      .
      (symbol) @module.builtin)
    (string_literal) @string.special
    (sequence
      .
      (string_literal) @string.special)
  ])
  (#any-of? @function.builtin
    "loadPackage" "installPackage" "uninstallPackage" "needsPackage" "export" "endPackage"
    "newPackage" "importFrom" "exportFrom"))

((binary_expression
  left: (symbol) @function.builtin
  operator: "_"
  right: (symbol) @module.builtin)
  (#any-of? @function.builtin "importFrom" "exportFrom"))
