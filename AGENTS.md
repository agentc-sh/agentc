# AGENTS Guidelines

This repository follows these guidelines for contributions by AI agents or humans:

1. **Commit Messages**: Use [Conventional Commits](https://www.conventionalcommits.org/) format. Examples include:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `test:` for test-related changes
   - `chore:` for maintenance tasks

2. **Simplicity First**: Prefer simpler implementations over overly complex solutions.

3. **Run Tests**: Always run tests before committing to ensure functionality and catch regressions. Use `cargo test` to run tests.

4. **Uniform Structure**: Maintain a consistent code structure across modules so files and packages are easy to navigate. Ensure code style is uniform.

5. **Explain Why**: Add comments explaining *why* something is done if it is not obvious from code alone.

6. **Documentation**: Update relevant documentation when making changes that affect usage or behavior. Ensure doc comments and docstrings are clear and follow Rust documentation conventions.

7. **Branch Names**: Use '_type_/_short_topic_' convention for new branches (e.g. feat/add-s3-backup).

9. **Style & Formatting**: Use `clippy` for linting and `rustfmt` for formatting to ensure code quality and consistency. Formatting throughout all of the code should stay consistent. Do not change existing code's format if it is already consistent with the project's style, unless explicitly requested.

10. **Security**: Run security checks using `cargo audit` to identify vulnerabilities in dependencies.

### For AI Agents

When working on a task, follow these steps:

1. **Understand the Task**: Read the task description carefully and ensure you understand the requirements.

2. **Plan Your Approach**: Outline the steps you will take to complete the task before writing code. Ensure your plan is documented as a markdown file in the agent's workspace following this structure:
   ```
   .agent/
      tasks/
         <task_name>/
            plan.md
            notes.md
   ```

   For more information on how to structure the plan, refer to the [PLANS](PLANS.md) guidelines.

3. **Implement the Solution**: Write code to implement your plan, following the guidelines above, strictly adhering to the plan.

4. **Document your thought process**: As you work through the task, document your thought process and any decisions you make in a `notes.md` file in the same directory as your `plan.md`. This will help others understand your reasoning and the steps you took to arrive at your solution.

5. **Test Your Code**: Run tests to ensure your implementation works as expected and does not introduce regressions.

6. **Document Your Changes**: Update documentation as needed to reflect any changes in functionality or usage.

## Code Style and Conventions

These rules are non-negotiable. Each is stated, then expanded with concrete examples.

### Rule 1 — Multiline over long single-line expressions

> "Multiline expressions are favored over long single line expressions"

Break argument lists, iterator chains, struct literals, and other complex expressions across multiple lines when they exceed ~80 columns or hide structure.

Avoid:

```rs
let result = match_host_url_patterns(record.id(), rows.iter().map(|r| r.value.trim()).filter(|v| !v.is_empty()).collect()).await?;
```

Prefer:

```rs
let result = match_host_url_patterns(
   record.id(),
   rows.iter()
      .map(|r| r.value.trim())
      .filter(|v| !v.is_empty())
      .collect(),
)
.await?;
```

### Rule 2 — No single-use intermediate variables

> "Do not assign variables only to use them once. Just build them where you pass them in that case."

Inline values used exactly once where they are consumed.

Avoid:

```rs
let request = MatchHostUrlPatternsRequest { paths };

let result = match_host_url_patterns(id, request).await?;
```

Prefer:

```rs
let result = match_host_url_patterns(
   id,
   MatchHostUrlPatternsRequest { paths },
)
.await?;
```

### Rule 3 — Comments are for public-API docstrings and vague logic

> "Comments are used to document public API with docstrings and explain vague logic that won't be understood at first glance with line comments. Do not write your life story with every possible thought you have in comments."

Default to no comments. Only write one when the reasoning is non-obvious.

Avoid:

```rs
// Create the response object.
let response = Response {
   paths,
   count: paths.len(),
};
```

Prefer:

```rs
/// Matches a collection of URL paths against host patterns.
pub async fn match_host_url_patterns(
   id: &str,
   paths: Vec<String>,
) -> Result<MatchResult> {
   // The API treats an empty path as a wildcard match.
   let normalized_paths = normalize_paths(paths);

   // ...
}
```

### Rule 4 — Logical line breaks group related statements

> "Use logical line breaks between lines to group related statements instead of squishing everything together like a mallet on a cheeseburger."

Group related statements with blank lines so the reader can see the phases.

Avoid:

```rs
pub async fn match_host_url_patterns_action(
   id: &str,
   paths: Vec<String>,
) -> Result<MatchHostUrlPatternsResult> {
   let result = match_host_url_patterns(id, paths).await?;
   log::info!("matched {} paths", result.matches.len());
   Ok(MatchHostUrlPatternsResult { matches: result.matches })
}
```

Prefer:

```rs
pub async fn match_host_url_patterns_action(
   id: &str,
   paths: Vec<String>,
) -> Result<MatchHostUrlPatternsResult> {
   let result = match_host_url_patterns(id, paths).await?;

   log::info!("matched {} paths", result.matches.len());

   Ok(MatchHostUrlPatternsResult {
      matches: result.matches,
   })
}
```

### Rule 5 — Prefer direct imports and consolidated `use` statements

> "Use aliases for internal imports to avoid long relative paths and make it clear when importing from within the project vs external dependencies."

> "Combine `use` statements rather than individually importing items from the same module, and always put `use` statements at the top of the file. Prefer importing paths instead of referencing items through their parent modules unless there are naming conflicts."

Avoid:

```rs
async fn handler() -> Result<()> {
   let err = crate::http::errors::ApiError::new("failed");

   crate::logging::logger::log_error(&err);

   Ok(())
}
```

Avoid:

```rs
use crate::http::errors::ApiError;
use crate::http::errors::ErrorCode;
use crate::http::errors::ErrorResponse;
```

Prefer:

```rs
use crate::{
   http::errors::{
      ApiError,
      ErrorCode,
      ErrorResponse,
   },
   logging::logger::log_error,
};

async fn handler() -> Result<()> {
   let err = ApiError::new("failed");

   log_error(&err);

   Ok(())
}
```

### Rule 6 — No reflexive type annotations

> "Inference is preferred. This means you should not write `let x: String = String::from("hello")` when `let x = String::from("hello")` suffices, or anything that repeats the type of the variable without adding clarity like casting with `as` when it is not needed. If explicit type annotations are needed, prefer using the turbofish syntax where the annotation is needed rather than on the variable declaration."

Avoid:

```rs
let name: String = String::from("hello");
```

Prefer:

```rs
let name = String::from("hello");
```

Avoid:

```rs
let items: Vec<i32> = (0..10).collect();
```

Prefer:

```rs
let items = (0..10).collect::<Vec<_>>();
```

### Rule 7 — Multiline expressions inbetween ( and ) should be on new lines

Any multiline expression that is wrapped in parentheses for whatever reason, such as Ok(...), Some(...), a function call, etc., should have the opening and closing parentheses on their own lines, with the contents of the expression indented one level.

Avoid:

```rs
Ok(some_field
   .some_method()
   .some_other_method())
```

Prefer:

```rs
Ok(
   some_field
      .some_method()
      .some_other_method()
)
```

