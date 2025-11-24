# Architecture and Design

The following considerations around specifics in the domain of Intel platforms
and firmware are meant to offer a deeper understanding of design choices.

## Goals

### Separate detection from action

`me_cleaner` mixes both, and assessing the exact actions to happen is hard.
Via its CLI, everything a one-shot operation, and the user needs to know the
options beforehand. Note that it is very Unixesque to wrap CLIs in shell scripts
as if they were intended as APIs. That is not meant here.

For programmatic and interactive use, provide an analysis result and let the
programmer/user _then_ choose what they want to do. The programmer has to deal
with all possibilities and may choose strategies around them, whereas the user
can benefit from seeing the options fitting their actual case.

E.g., in a GUI app such as [Fiedka](https://fiedka.app) or a TUI that could be
created with [ratatui](https://ratatui.rs/), setting a HAP bit vs legacy
(Alt)MeDisable bit(s) can be elaborated on in a help message or tooltip.
Removable partitions and files can be listed rather than expected to be provided
by the user who, before the analysis, doesn't know what their firmware contains.

### Comprehensible and useful API

Provide an API that requires little knowledge of internal details.
The job of an API is to simplify, after all. Otherwise, both caller and callee
would duplicate efforts.

Encapsulate rather than exposing many raw structs directly.
Make relationships and containment clear.

E.g., wrap partition entry structs in another struct that carries supporting
metadata with it, such as offset and size, parsing result of what the partition
entry points to, etc..

Follow the [guidelines for extending `intel_fw`](./extend.md).

## Approaches

Replies to a [StackOverflow question on separating preparation from execution](
https://stackoverflow.com/questions/4355263/is-there-a-design-pattern-for-separating-preparation-of-work-from-execution)
reference the [Composite pattern](https://en.wikipedia.org/wiki/Composite_pattern)
and the [Command pattern](https://rust-unofficial.github.io/patterns/patterns/behavioural/command.html).

The Composite pattern helps in that there is a hierarchical structure, i.e.,
IFD, FIT and ME firmware on a high level, and then IFD has its underlying data,
the ME firmware has its FPT and partitions therein, and the FIT points to many
things. Underlying details are very different though, so there are no meaningful
abstractions for common interfaces.

The Command pattern may make sense on a higher level, for example, to collect
the actions resulting from command line switches or API options. At the same
time, it creates more boilerplate code, which may be hard to follow.

Note that we need to check the applicability of operations on two levels:
Some operations exclude others, and some only apply if their assumptions fit
with findings from analyzing the given firmware image.

[Fiano/utk](https://github.com/linuxboot/fiano) implements the [visitor pattern](
https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html),
which is meant for objects that are quite similar in some regard.

One issue with firmware images is that while many things are related to each
other, they also have their very specific purposes and semantics, and there is a
rather fixed structure. It would thus be simpler to **deal with specific
operations on a higher level that understands the domain and carries clear
semantics** with it rather than having a visitor/executor encoding each possible
operation for each bit and requiring more understanding from the user and
programmer.
I.e., just have **each entity offer its own unique set of clearly understandable
helpers** and assisting parameters, possibly following a loose pattern when
multiple entities are similar.

For example, some partitions contain directories in the case of generation 2 and
generation 3 ME:
For both generations, certain directories can be partially cleared (whole ranges
of data set to `0xff`), and both can take a list of things to retain.
That requires helpers which may implement a common trait (interface).
Similarly, other partitions can be cleared, but _entirely_, so they do not need
extra functionality themselves. Trying to force the same interface on them only
results in unnecessary extra code.

On the other hand, the FPT itself is a small memory slice (found to be <2K), so
it can take care of itself. It has a checksum in its header. Offer helpers for
editing and returning a new slice to replace the original one.
The IFD is similar in that it is also small, but has its own unique features.
And the FIT is yet again very different.

See also the [strategy pattern](
https://rust-unofficial.github.io/patterns/patterns/behavioural/strategy.html).
