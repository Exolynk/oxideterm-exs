// External scanner for ExS hash-delimited raw strings.
//
//   r#"..."#   raw string           d#"..."#   dedented raw string
//   f#"..."#   formatted raw string fd#"..."#  dedented formatted raw string
//
// Any number of `#` may be used; the string ends at `"` followed by the same
// number of `#`. Formatted variants interpolate `{expr}` and treat `{{`/`}}`
// as literal braces, so their content stops before `{` and `}`.

#include "tree_sitter/alloc.h"
#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>

enum TokenType {
  RAW_STRING_START,
  FORMATTED_RAW_STRING_START,
  RAW_STRING_CONTENT,
  RAW_STRING_END,
  ERROR_SENTINEL,
};

typedef struct {
  uint8_t hash_count;
  bool formatted;
} Scanner;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

void *tree_sitter_exs_external_scanner_create(void) {
  Scanner *scanner = ts_calloc(1, sizeof(Scanner));
  return scanner;
}

void tree_sitter_exs_external_scanner_destroy(void *payload) { ts_free(payload); }

unsigned tree_sitter_exs_external_scanner_serialize(void *payload, char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  buffer[0] = (char)scanner->hash_count;
  buffer[1] = (char)scanner->formatted;
  return 2;
}

void tree_sitter_exs_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  scanner->hash_count = 0;
  scanner->formatted = false;
  if (length == 2) {
    scanner->hash_count = (uint8_t)buffer[0];
    scanner->formatted = buffer[1] != 0;
  }
}

// Reads `#...#"` after the prefix letter(s) have been consumed.
static bool scan_start(Scanner *scanner, TSLexer *lexer, bool formatted, enum TokenType symbol) {
  uint8_t hashes = 0;
  while (lexer->lookahead == '#') {
    if (hashes == UINT8_MAX) return false;
    hashes++;
    advance(lexer);
  }
  if (hashes == 0 || lexer->lookahead != '"') return false;
  advance(lexer);
  scanner->hash_count = hashes;
  scanner->formatted = formatted;
  lexer->result_symbol = symbol;
  return true;
}

static bool scan_start_token(Scanner *scanner, TSLexer *lexer, const bool *valid_symbols) {
  bool raw = valid_symbols[RAW_STRING_START];
  bool formatted = valid_symbols[FORMATTED_RAW_STRING_START];
  if (!raw && !formatted) return false;

  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\n' ||
         lexer->lookahead == '\r') {
    lexer->advance(lexer, true);
  }

  switch (lexer->lookahead) {
    case 'r':
    case 'd':
      if (!raw) return false;
      advance(lexer);
      return scan_start(scanner, lexer, false, RAW_STRING_START);
    case 'f':
      if (!formatted) return false;
      advance(lexer);
      if (lexer->lookahead == 'd') advance(lexer);
      return scan_start(scanner, lexer, true, FORMATTED_RAW_STRING_START);
    default:
      return false;
  }
}

// Scans content up to the closing delimiter, which it emits as a separate token.
static bool scan_content_or_end(Scanner *scanner, TSLexer *lexer, const bool *valid_symbols) {
  bool has_content = false;
  for (;;) {
    if (lexer->eof(lexer)) {
      // Unterminated: keep what we have as content so highlighting continues.
      if (has_content && valid_symbols[RAW_STRING_CONTENT]) {
        lexer->mark_end(lexer);
        lexer->result_symbol = RAW_STRING_CONTENT;
        return true;
      }
      return false;
    }

    if (lexer->lookahead == '"') {
      lexer->mark_end(lexer);
      advance(lexer);
      uint8_t hashes = 0;
      while (lexer->lookahead == '#' && hashes < scanner->hash_count) {
        hashes++;
        advance(lexer);
      }
      if (hashes == scanner->hash_count) {
        if (has_content) {
          lexer->result_symbol = RAW_STRING_CONTENT;
          return valid_symbols[RAW_STRING_CONTENT];
        }
        if (!valid_symbols[RAW_STRING_END]) return false;
        lexer->mark_end(lexer);
        scanner->hash_count = 0;
        scanner->formatted = false;
        lexer->result_symbol = RAW_STRING_END;
        return true;
      }
      // Not the delimiter: the quote and hashes belong to the content.
      has_content = true;
      continue;
    }

    if (scanner->formatted && (lexer->lookahead == '{' || lexer->lookahead == '}')) {
      if (!has_content || !valid_symbols[RAW_STRING_CONTENT]) return false;
      lexer->mark_end(lexer);
      lexer->result_symbol = RAW_STRING_CONTENT;
      return true;
    }

    advance(lexer);
    has_content = true;
  }
}

bool tree_sitter_exs_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;

  // During error recovery every symbol is valid; let the internal lexer work.
  if (valid_symbols[ERROR_SENTINEL]) return false;

  if (valid_symbols[RAW_STRING_CONTENT] || valid_symbols[RAW_STRING_END]) {
    return scan_content_or_end(scanner, lexer, valid_symbols);
  }

  return scan_start_token(scanner, lexer, valid_symbols);
}
