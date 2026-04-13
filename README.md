# litter-dox

Clean literate programming for Rust, without the odour.

Create simple relations between your code and your documentation.

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

Users may insert an anchor manually where they insert a link in documentation,
or let `litter_dox::litter_anchors!()` find links in documatation and automatically add anchors.

## Example

<a id="fibonacci"></a>[`Fibonacci_n` as found in the integration test](litdox/fibonacci.md)

## Acknowledgement

litter-dox is inspired by [Johan Hidding](<https://www.github.com/jhidding>)'s continuous advocacy for literate programming and his work on [entangled](<https://entangled.github.io/>).

If you wish to employ literate programming for anything remotely serious, I encourage you to look into `entangled` thinking about this crate.

## LLM use

Gemini Flash 3 helped as teacher, sparring partner and knowledge base.

## License

MIT
