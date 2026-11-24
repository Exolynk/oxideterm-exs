; Highlight queries for ExS (Exolynk Xtension Script), https://github.com/Exolynk/exs
;
; Categories follow the ExS compiler's highlighter (exs_compiler::highlight),
; which the Exolynk platform IDE uses. Code inside `f"{...}"` interpolations is
; highlighted as code.
;
; OxideTerm paints the FIRST capture covering a byte, so specific patterns
; come before generic ones (the reverse of the Neovim convention).

; ---- comments --------------------------------------------------------------

(line_comment) @comment
(block_comment) @comment

; ---- strings ---------------------------------------------------------------

; Quoted object keys: `{ "content-type": value }`.
(_
  (string ["\"" "b\"" (string_content) (escape_sequence)] @property)
  .
  (punctuation) @_colon
  (#eq? @_colon ":"))

(escape_sequence) @string.escape
(brace_escape) @string.escape
(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

(string ["\"" "b\""] @string)
(string (string_content) @string)
(formatted_string ["f\"" "\""] @string)
(formatted_string (string_content) @string)
(raw_string) @string
(formatted_raw_string (raw_string_content) @string)
(escaped_string) @string

; ---- literals --------------------------------------------------------------

(number) @number

; ---- definitions -----------------------------------------------------------

(function_definition "fn" @keyword.function)
(function_definition name: (identifier) @function)

(parameters
  (parameter) @type
  (#match? @type "^[A-Z]"))
(parameter) @variable.parameter
(parameters (scoped_identifier (identifier) @type))
(parameters (none) @type)
(parameters (unit) @type)
(parameters (builtin_type) @type)

(let_binding "let" @keyword)
(let_binding name: (identifier) @variable)

; ---- contracts -------------------------------------------------------------

; `-> Int | None`, `field: String | Error`: built-ins in contract position.
(_
  (operator) @_op
  .
  [(none) (unit) (builtin_type)] @type
  (#any-of? @_op "->" "|"))
(_
  (operator) @_op
  .
  (identifier) @type
  (#any-of? @_op "->" "|")
  (#match? @type "^[A-Z]"))
(_
  (operator) @_op
  .
  (scoped_identifier (identifier) @type)
  (#any-of? @_op "->" "|"))

; ---- calls, paths and members ----------------------------------------------

(call function: (identifier) @function)
; `Error("Kind", "message", data)` constructs an Error value.
(_
  (builtin_type) @function
  .
  (punctuation) @_paren
  (#eq? @_paren "("))
(scoped_call function: (scoped_identifier (identifier) @function))
(method_call name: (identifier) @function.method)
(field_access name: (identifier) @property)

; `import exo::ics;` and `use log::{info};` name modules and types.
(_
  (keyword) @_import
  .
  (scoped_identifier (identifier) @type)
  (#any-of? @_import "import" "use"))

(scoped_identifier "::" @punctuation.delimiter)
(scoped_identifier (identifier) @variable)

(object_key name: (identifier) @property)

; ---- keywords --------------------------------------------------------------

(keyword) @keyword
(reserved_word) @keyword
(boolean) @keyword
(none) @keyword
(unit) @keyword
(builtin_type) @keyword

; ---- identifiers -----------------------------------------------------------

; ExS colors constants (`MAX_ITEMS`) like other variables.
((identifier) @variable
  (#match? @variable "^[A-Z][A-Z0-9_]+$"))

((identifier) @type
  (#match? @type "^[A-Z]"))

(identifier) @variable

; ---- operators and punctuation ---------------------------------------------

((operator) @punctuation
  (#any-of? @punctuation ".." "..=" "..."))
(operator) @operator
(method_call "." @punctuation.delimiter)
(field_access "." @punctuation.delimiter)
(call "(" @punctuation.bracket)
(method_call "(" @punctuation.bracket)
(scoped_call "(" @punctuation.bracket)
(parameters ["(" ")"] @punctuation.bracket)
(object_key ":" @punctuation.delimiter)
(block ["{" "}"] @punctuation.bracket)
(punctuation) @punctuation
(source_file ["{" "}"] @punctuation.bracket)
