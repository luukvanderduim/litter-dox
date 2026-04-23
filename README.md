# litter-dox

Clean [literate programming](https://en.wikipedia.org/wiki/Literate_programming) for Rust programmers, without the odour.

Create simple and convenient relations between documentation and code.

## Demo

In `/tests/demo.rs` <a id="fibonacci"></a>[a fibonacci function](litdox/fibonacci.md) is defined to compute the n-th number of the series.

The link above will remain up-to-date, even if the code itself is updated.

## Usage

Include litter-dox in your `Cargo.toml`.

```toml
[dependencies]
litter-dox = "0.1.0"
```

`litter-dox` provides two macros to help refer to code fragments and back to documentation:
1. `#{litter}`
2. `litter_acnhors!`

### Code fragments

Decorating an item with `#[litter]` will create a formatted markdown file of the fragment in your project.

```Rust
use litter_dox::litter;

#[litter]
fn bazify(input: &str) -> bool {
    // bazzing here
}
```

And in your projects `README.md` you can refer to your code fragment as follows:

```md
[implementation of bazify](./dox/bazify.md) 
```

note: litter-dox supports "README.md", "readme.md", "Readme.md" or "ReadMe.md" as return document,
but users can specify their own using the `doc` argument.

#### litter arguments

`#[litter]` accepts two arguments  "name" and "doc".

note: litter supports both ':' and '=' to assign a value.


##### Name

Without arguments, litter will take the item's identifier as fragment name.
Obviously this will not work for fragments without identifiers.

```Rust
#[litter]
struct 2D_Point {
    x: i32,
    y: i32,
}
```

```Rust
#[litter(name = "print_calculation")]
{
    // calculation
    println!("{}, 6.8_f32);
}
```
##### Doc

Because markdown does not support inlining markdown files, markdown fragment files include a 'return to documentation' link.

Specify a return document other than the default `README.md`.
```Rust
#[litter(doc: "Code.md")]
struct Bar;
```

#### Anchors

Users may insert an anchor manually `<a id="fragment-name-here"></a>` wherever they find it convenient to link to.
or allow <a id="litter_anchors"></a>[`litter_dox::litter_anchors!()`](litdox/litter_anchors.md) find links in documatation and automatically add a matching anchor before each link.


## Acknowledgement

litter-dox is provoked by [Johan Hidding](<https://www.github.com/jhidding>)'s continuous advocacy for literate programming with [entangled](<https://entangled.github.io/>).

If you wish to engage with literate programming for anything remotely serious, you are encouraged to look into `entangled` before considering this crate.

## LLM use

Gemini Flash 3 was useful as teacher, sparring partner and knowledge base.

## License

Licensed under either of

    Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

    MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
