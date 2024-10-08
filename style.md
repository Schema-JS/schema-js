# SchemaJS Style Guide

### Copyright Headers

Most modules in the repository should have the following copyright header:

> // Copyright 2018-2024 the SchemaJS authors. All rights reserved. MIT license.

If the code originates elsewhere, ensure that the file has the proper copyright headers. We only allow MIT, BSD, and Apache licensed code.

### Use underscores, not dashes in filenames 

Example: Use `file_and_something.ts` instead of `file-and-something.ts`.

### Add tests for new features

Each module should contain or be accompanied by tests for its public functionality.

### TODO Comments

TODO comments should usually include an issue or the author's github username in parentheses. Example:

```
// TODO(ap): Add tests.
// TODO(#123): Fix ticket.
// FIXME(#456): Implement feature.
```

### Rust

- Follow Rust conventions and be consistent with existing code.
- Crates should strive to follow the single responsability principle.


