<div align="center">
  <h1>litter-dox</h1>
  <p>Clean literate programming for Rust programmers, without the odour.</p>
</div>

Clean [literate programming](https://en.wikipedia.org/wiki/Literate_programming) for Rust programmers, without the odour.

Create simple and convenient relations between documentation and code.  

`litter-dox` keeps your documentation and code in sync by generating formatted Markdown fragments from your source code. It automates the "roundtrip" between your `README.md` and your implementation, ensuring links never break and code snippets are always up-to-date.


---  
## Index 

- [Demo](#demo)
- [Usage](#usage)
  - [Code Fragments](#code-fragments)
    - [`litter` arguments](#litter-arguments)
      - [Name](#name)
      - [Doc](#doc)
    - [Anchors](#anchors)
    - [Environment variables](#environment-variables)
- [Acknowledgement](#acknowledgement)
- [LLM use](#llm-use)
- [License](#license)
- [Contribution](#contribution)

---  


## Demo

In `/tests/demo.rs` <a id="fibonacci"></a>[a fibonacci function](litdox/fibonacci.md) is defined to compute the n-th number of the series.

The link to the snippet will show the most current code, regardless of whether the code has changed.
This is achieved by versioning the fragment with its hash on disk. The on-disk fragment gets updated only if the hash of the AST-fragment differs.

## Usage

First add `litter-dox` to dependncies in `Cargo.toml`.

```toml
[dependencies]
litter-dox = "0.1.0"
```

`litter-dox` provides two macros to help refer to code fragments and back to documentation:
1. `#[litter]`
2. `litter_anchors!`

### Code fragments

Decorating an item with `#[litter]` will create a formatted markdown file of the fragment in your project.

```Rust
use litter_dox::litter;

#[litter]
fn fibonacci_n(n: u32) -> u32 {
...
}
```

In project docs, `README.md` by default, one can now refer to that fragment by name:

```md
[implementation of fibonacci_n](./dox/fibonacci_n.md) 
```

note: `litter-dox` supports "README.md", "readme.md", "Readme.md" or "ReadMe.md" as return document,
but users can specify their own using the `doc` argument.

#### `litter` arguments

`#[litter]` accepts two arguments  `name` and `doc`.  
note: litter supports both ':' and '=' to assign a value to `name` or `doc`.

##### Name

The name argument may be used to name the fragment.  

Without a `name` argument, litter will take the item's identifier as fragment name.
Obviously this will not work for fragments without identifiers.

```Rust
#[litter]
struct 2D_Point {
    x: i32,
    y: i32,
}
```

```Rust
#[litter(name = "print_value")]
{
    println!("{}, 6.8_f32);
}
```
##### Doc

Markdown does not support inlining markdown files. This is why markdown fragment files include a 'return to documentation' link to make the roundtrip convenient.
The `doc` argument allows users to specify which document to return to.

```Rust
#[litter(doc: "Code.md")]
struct Bar;
```

#### Anchors

Users can either manually add fragment-link anchors, or use a macro to have anchors added to links automatically.  
Manually adding anchor looks like this:
`<a id="fibonacci"></a>`  

or add `litter_dox::litter_anchors!()` in the a source file.

```rust
litter_dox::litter_anchors!();
```

This function-like proc-macro will look for links matching existing fragments and add the appropriate anchor - provided the anchor does not already exist in the document.
This flexibility allows users to move the generated anchor around to wherever they deem it fitting.

#### Environment variables

As per default, fragments are stored in `litdox` in the projects manifest root.  
Users can change this to their preferred path by setting `LITTER_DOX_PATH`.

## Acknowledgement

[Johan Hidding](<https://www.github.com/jhidding>)'s continuous advocacy for literate programming elicited question if literate programming was feasible using proc macros in Rust.

If you wish to engage with literate programming for anything remotely serious, you are encouraged to look into his [entangled](<https://entangled.github.io/>) project before considering this crate.

## LLM use

Gemini Flash 3 was useful as teacher, sparring partner and knowledge base.

## License

Licensed under either of

    Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

    MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
