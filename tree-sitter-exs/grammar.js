/**
 * Tree-sitter grammar for ExS (Exolynk Xtension Script).
 * Language: https://github.com/Exolynk/exs
 *
 * The grammar is intentionally lexical: a source file is a flat sequence of
 * tokens with a few small structures (call heads, definitions, member access,
 * paths, object keys, strings with interpolation). It mirrors the token rules
 * of `exs-compiler` (src/lexer.rs) and the categories of
 * `exs::tooling::HighlightKind`, and stays robust for half-written code in an
 * editor because nearly every token sequence is valid.
 */

const KEYWORDS = [
  'as', 'break', 'const', 'continue', 'else', 'enum', 'fn', 'for', 'if',
  'impl', 'import', 'in', 'is', 'let', 'match', 'par', 'private', 'ret',
  'trait', 'type', 'use', 'while',
];

// Reserved by the lexer without a meaning yet (E0011).
const RESERVED = ['async', 'await', 'loop', 'return', 'struct', 'yield'];

const OPERATORS = [
  '->', '=>', '...', '..=', '..', '==', '!=', '<=', '>=', '&&', '||',
  '+', '-', '*', '/', '%', '!', '=', '<', '>', '|', '?',
];

const PUNCTUATION = ['(', ')', '[', ']', ',', ';', '::', ':', '.'];

const PREC = {
  head: 1,
  call: 2,
};

module.exports = grammar({
  name: 'exs',

  extras: $ => [/\s/, $.line_comment, $.block_comment],

  word: $ => $.identifier,

  externals: $ => [
    $._raw_string_start,
    $._formatted_raw_string_start,
    $.raw_string_content,
    $._raw_string_end,
    $._error_sentinel,
  ],

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice($._flat_item, '{', '}'),

    // Everything except bare braces, so interpolations can balance them.
    _flat_item: $ => choice(
      $.string,
      $.formatted_string,
      $.raw_string,
      $.formatted_raw_string,
      $.escaped_string,
      $.number,
      $.boolean,
      $.none,
      $.unit,
      $.builtin_type,
      $.function_definition,
      $.let_binding,
      $.call,
      $.method_call,
      $.field_access,
      $.scoped_call,
      $.scoped_identifier,
      $.object_key,
      $.keyword,
      $.reserved_word,
      $.identifier,
      $.operator,
      $.punctuation,
    ),

    // ---- comments -------------------------------------------------------

    line_comment: _ => token(seq('//', /[^\n]*/)),

    block_comment: _ => token(seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')),

    // ---- literals -------------------------------------------------------

    number: _ => token(/\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d[\d_]*)?/),

    boolean: _ => choice('true', 'false'),
    none: _ => 'None',
    unit: _ => 'Unit',
    builtin_type: _ => choice('Error', 'Host'),

    string: $ => seq(
      choice('"', 'b"'),
      repeat(choice($.string_content, $.escape_sequence)),
      token.immediate('"'),
    ),

    formatted_string: $ => seq(
      'f"',
      repeat(choice(
        alias($._formatted_content, $.string_content),
        $.escape_sequence,
        $.brace_escape,
        $.interpolation,
      )),
      token.immediate('"'),
    ),

    raw_string: $ => seq(
      $._raw_string_start,
      optional($.raw_string_content),
      $._raw_string_end,
    ),

    formatted_raw_string: $ => seq(
      $._formatted_raw_string_start,
      repeat(choice($.raw_string_content, $.brace_escape, $.interpolation)),
      $._raw_string_end,
    ),

    // `\"name\"` inside an interpolation of a formatted string.
    escaped_string: _ => token(seq('\\"', /([^"\\\n]|\\[^"\n])*/, '\\"')),

    string_content: _ => token.immediate(prec(1, /[^"\\\n]+/)),

    _formatted_content: _ => token.immediate(prec(1, /[^"\\{}\n]+/)),

    escape_sequence: _ => token.immediate(prec(1, /\\(u\{[0-9A-Fa-f]+\}|.)/)),

    brace_escape: _ => token.immediate(prec(2, choice('{{', '}}'))),

    interpolation: $ => seq(
      token.immediate('{'),
      repeat(choice($._flat_item, $.block)),
      '}',
    ),

    block: $ => seq('{', repeat(choice($._flat_item, $.block)), '}'),

    // ---- small structures -----------------------------------------------

    // `fn name(params)`; the parameter list is optional so that a
    // half-typed `fn name` still parses cleanly.
    function_definition: $ => prec.right(PREC.head, seq(
      'fn',
      field('name', $.identifier),
      optional($.parameters),
    )),

    parameters: $ => seq(
      '(',
      repeat(choice(
        $.parameter,
        $.scoped_identifier,
        $.none,
        $.unit,
        $.builtin_type,
        alias(choice('|', '...', '?'), $.operator),
        alias(choice(',', ':'), $.punctuation),
      )),
      ')',
    ),

    // Lowercase names are parameters, capitalized names are contract types.
    parameter: $ => $.identifier,

    let_binding: $ => prec(PREC.head, seq(
      'let', field('name', $.identifier),
    )),

    call: $ => prec(PREC.call, seq(
      field('function', $.identifier), '(',
    )),

    method_call: $ => prec(PREC.call, seq(
      '.', field('name', $.identifier), '(',
    )),

    field_access: $ => prec(PREC.head, seq(
      '.', field('name', $.identifier),
    )),

    // `a::b::c` as one unit, like the ExS highlighter colors whole paths.
    scoped_identifier: $ => prec.right(PREC.head, seq(
      $.identifier,
      $._path_tail,
    )),

    // Right-recursive so the longest path wins; a trailing `::` stays valid
    // for `use log::{info};`.
    _path_tail: $ => prec.right(seq(
      '::',
      optional(seq($.identifier, optional($._path_tail))),
    )),

    scoped_call: $ => prec(PREC.call, seq(
      field('function', $.scoped_identifier), '(',
    )),

    object_key: $ => prec(PREC.head, seq(
      field('name', $.identifier), ':',
    )),

    // ---- plain tokens ---------------------------------------------------

    keyword: _ => choice(...KEYWORDS),

    reserved_word: _ => choice(...RESERVED),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,

    operator: _ => choice(...OPERATORS),

    punctuation: _ => choice(...PUNCTUATION),
  },
});
