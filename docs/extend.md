# Extend with new variants and features

These guiding principles are meant to help extending `intel_fw` over time.
As a library, it should adhere to those as requirements in order to provide
guarantees to applications working with it. Generally, it should follow the
[Rust API guidelines](https://rust-lang.github.io/api-guidelines/about.html).

## Do not panic!

Parsing firmware means looking for and acting on offsets and sizes frequently,
and they always need to be checked to stay within the bounds of the given data.
Never `.unwrap()` or `.expect()`, which mean an _intentional panic_, in case
something cannot be found, read, or recognized. Instead, return instances of
`Self` for structs, wrapped in a `Result<Self, CustomError>` or possibly
`Option<Self>`. Do not `assert!()` or explicitly `panic!()` either.

## `Option` or `Result`

In general, use `Option<Self>` for things that might not be found. For example,
a full image from a production system would have an IFD at the beginning,
whereas an ME-only image would not. On the other hand, either image is expected
to contain an FPT, so in this case, provide a `Result<Self, CustomError>`,
because not finding an FPT means that something is clearly wrong with the image.

## Continuous parsing

There are circumstances under which a parser encounters an issue. It can be
that offsets do not make sense, magic bytes (struct markers) are not as they
were expected, or new variants are found with samples not encountered before.
In those situations, it is desirable to follow a best-effort strategy. I.e.,
when there is still remaining data that could be parsed, keep going.

## Let apps provide feedback

Finally, **let the consuming application take care** of taking the result apart.
Nested structures can be thought of as trees, similar to ASTs in programming
languages. Whenever a node in the tree that is a parsing result turns into an
`Error` or `None`, other nodes beside it may still provide useful information.
That information helps the user of an app to understand the context and possibly
report what they are facing or look into the issue themselves.

## Errors and context

In order to make sense of an error, context is important. During development, it
can be helpful to print out information right within a specific parser. However,
a final app is typically not run in development mode, but as a release. In that
moment, semantic errors will help to identify possible problems. Include offsets
and sizes (or `Range`s) for the application to tell exactly where the problem
is, and it can choose to e.g. dump a contextual hex view on the data.

## Additional information

To provide additional information, instead of returning an `Err(String)`, wrap 
it in an `enum CustomError`, i.e., for example, 
```rs
enum CustomError {
    TooSmall(String),
}
```
so you would `return Err(CustomError:TooSmall(format!("{n{ bytes needed")))`.
