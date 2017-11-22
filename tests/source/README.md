# Source Tests

This folder contains `rustdoc`'s "source tests": tests that verify `rustdoc`
outputs JSON as expected. The Rust files in this folder are scanned for test
directive comments that are later run as tests.

The `rustdoc` build script will generate a test case that tests these directives
for each Rust file in this folder.

## Directive Syntax

### `@has`

```rust
// @has <JMESPath> <Regular expression>
```

The `@has` directive requires that the given [JMESPath] query executed on
`rustdoc`'s JSON output evaluates to a string that matches the given regular
expression.

The test will fail if any of the following conditions are met:

- The JMESPath or regular expression syntax is invalid.
- The JMESPath query does not evaluate to a string.
    - If the test is negated and the query matches a null value, the test will
      succeed.
- The regular expression does not match the JMESPath query result and the test
  is not negated.

### `@matches`

```rust
// @matches <JMESPath> <JSON>
```

The `@matches` directive requires that the result of the JMESPath query executed
on `rustdoc`'s JSON output equals the JSON on the right. This can be used to
form more complex queries than `@has`.

The test will fail if any of the following conditions are met:

- The JMESPath or JSON syntax is invalid.
- The JMESPath query result does not match the JSON value and the test is not
  negated.

### `@assert`

```rust
// @assert <JMESPath>
```

`@assert` is a shorthand for matching `true`.

The test will fail if any of the following conditions are met:

- The JMESPath syntax is invalid.
- The JMESPath does not evaluate to a boolean.
- The JMESPath evaluates to `false` and the test is not negated.

### Additional Notes

Arguments are quoted according to shell rules. Arguments containing spaces must
be enclosed in quotes.

Long tests may span multiple lines by ending the line with a `\`.

```rust
// @has a.very.long.jmespath \
//     'a very long line of documentation'
```

Each directive supports negation by prefixing with `!`. For example, `@!has path 'value'` would
fail if the JMESPath evaluated to `"value"`.

[JMESPath]: http://jmespath.org/

