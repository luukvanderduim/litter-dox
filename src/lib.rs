use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::LazyLock,
};
use syn::{Item, LitStr, Token, parse_macro_input};

type Result<T> = std::result::Result<T, syn::Error>;

// Where the project's manifest directory is located - with fallback.
static PROJECT_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
});

static README_VARIANTS: &[&str] = &["README.md", "readme.md", "Readme.md", "ReadMe.md"];
static README: LazyLock<Option<String>> = LazyLock::new(|| {
    README_VARIANTS
        .iter()
        .find(|&&v| Path::new(v).exists())
        .map(|&v| v.to_string())
});

/// This is set to the `LITTER_DOX_PATH` env variable, `"litdox"` or PROJECT_ROOT in this order.
static FRAGMENT_DIR: LazyLock<FragmentDir> = LazyLock::new(|| {
    let path = std::env::var("LITTER_DOX_PATH")
        .ok() // -> Option
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            let local = PROJECT_ROOT.join("litdox");
            if local.exists() {
                local
            } else {
                PROJECT_ROOT.clone()
            }
        });

    let dir_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .expect("path always resolves to path");

    FragmentDir {
        dir_path: path,
        escaped_dir_name: regex::escape(&dir_name),
    }
});

/// The directory where code fragment files are stored and the escaped folder name for use in regex patterns.
/// This is set to the `LITTER_DOX_PATH` env variable, or `"litdox"` if that is not set or if all else fails PWD `"."` is used.
struct FragmentDir {
    /// Path to the directory where code fragment files are stored.
    dir_path: PathBuf,

    /// Regex escaped dir name.
    escaped_dir_name: String,
}

// A type to encapsulate the parsed relevant input.
enum LitterFragment {
    /// Multiple items or a whole module structure.
    Module(Box<syn::File>),

    /// A single named Rust item (struct, fn, enum, etc.)
    Item(Box<syn::Item>),

    /// Raw tokens for blocks or expressions.
    Fragment(Box<TokenStream2>),
}

impl LitterFragment {
    fn to_formatted_string(&self) -> String {
        match self {
            LitterFragment::Module(f) => prettyplease::unparse(f),

            LitterFragment::Item(i) => {
                // We wrap the item in a temporary File so prettyplease can handle it
                let file = syn::File {
                    shebang: None,
                    attrs: vec![],
                    items: vec![(**i).clone()],
                };
                prettyplease::unparse(&file)
            }

            LitterFragment::Fragment(tokens) => {
                // For raw fragments, we use the string representation.
                // Since the TokenStream is "sealed" (braces included), it remains valid.
                tokens.to_string()
            }
        }
    }
}

impl Hash for LitterFragment {
    fn hash<F: Hasher>(&self, state: &mut F) {
        match self {
            LitterFragment::Module(f) => f.hash(state),
            LitterFragment::Item(i) => i.hash(state),
            LitterFragment::Fragment(tokens) => tokens.to_string().hash(state),
        }
    }
}

// The litter attribute can take a name and/or a return document argument.
//
// ```
// #[litter(name = "foo", doc = "CodeOverview.md")]
// struct Foo
//
// #[litter(name: "foo", doc: "CodeOverview.md")]
// struct Bar
// ```
struct ParsedAttributes {
    // The name users refer to in markdown link.
    // Defaults to the item's identifier if it has one.
    fragment_name: Option<String>,

    // Return document name.
    // Defaults to `README.md` if it exists.
    return_doc: Option<String>,
}

impl syn::parse::Parse for ParsedAttributes {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut fragment_name = None;
        let mut return_doc = None;

        while !input.is_empty() {
            let ident = input.parse::<Ident>()?;
            match ident.to_string().as_str() {
                "name" => {
                    if input.peek(Token![:]) {
                        input.parse::<Token![:]>()?;
                    } else {
                        input.parse::<Token![=]>()?;
                    }
                    let lit = input.parse::<LitStr>()?;
                    fragment_name = Some(lit.value());
                }
                "doc" => {
                    if input.peek(Token![:]) {
                        input.parse::<Token![:]>()?;
                    } else {
                        input.parse::<Token![=]>()?;
                    }
                    let lit = input.parse::<LitStr>()?;
                    return_doc = Some(lit.value());
                }

                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "Unexpected argument: {ident}. Supported arguments for `#[litter]` are 'name' and/or 'doc'. Both '=' and ':' are supported for assignment."
                        ),
                    ));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ParsedAttributes {
            fragment_name,
            return_doc,
        })
    }
}

#[proc_macro_attribute]
pub fn litter(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Rustc provides `proc-macro::TokenStream` which becomes available when the library is set to be of proc-macro type.
    // `proc-macro::TokenStream` however is an opaque type, `proc_macro2::TokenStream` is a more ergonomic and malleable rendition.
    let item2 = TokenStream2::from(item.clone());

    // Obtain user supplied args, if any.
    let attr_args = parse_macro_input!(attr as ParsedAttributes);

    // Determine the return doc name (if any) or handle error
    let return_doc_name = if let Some(rd) = attr_args.return_doc {
        rd
    } else if let Some(rd) = &*README {
        rd.clone()
    } else {
        return comp_error(
            &item2,
            "No 'README.md' found in project root. Either create one or specify a return document using #[litter(doc = \"Other.md\")].",
        );
    };

    // Parse to name and input into litter relevant scope.
    let (fragment_name, parsed_input) = match (
        attr_args.fragment_name,
        syn::parse2::<syn::File>(item2.clone()),
        syn::parse2::<syn::Item>(item2.clone()),
    ) {
        // Name provided, and it's a full file / capture of several items.
        (Some(n), Ok(f), _) => (n, LitterFragment::Module(Box::new(f))),

        // Name provided, and it's an item
        (Some(n), _, Ok(i)) => (n, LitterFragment::Item(Box::new(i))),

        // No name, but we may have a named `Item`
        (None, _, Ok(i)) => {
            if let Some(ident) = get_item_ident(&i) {
                (ident.to_string(), LitterFragment::Item(Box::new(i)))
            } else {
                return comp_error(
                    &item2,
                    "A name is required for this type of item (e.g. impl blocks).",
                );
            }
        }

        // Name provided, but parsing as File/Item failed (it's a fragment)
        (Some(n), _, _) => (n, LitterFragment::Fragment(Box::new(item2.clone()))),

        // Error Cases: No name provided for anonymous structures
        (None, Ok(_), _) => {
            return comp_error(&item2, "Module-level (#![litter]) requires a name.");
        }
        _ => {
            return comp_error(
                &item2,
                "Code fragments require a name: #[litter(\"name\")].",
            );
        }
    };

    // The fragment file will contain a short hash to version the fragment.
    let mut hasher = DefaultHasher::new();
    parsed_input.hash(&mut hasher);
    // Mask against 28 LSBs, 4 bits equals a hex digit, thus 7 hex digits
    let hash = hasher.finish() & 0x0FFF_FFFF;

    // Format after hashing to avoid a hash dependency on `prettyplease` dependency
    // so we can update `prettyplease` without falsely invalidating fragment hashes.
    let formatted_fragment = parsed_input.to_formatted_string();
    let fragment_file_name = format!("{}.md", fragment_name);
    let fragment_file_path =
        std::path::PathBuf::from(&FRAGMENT_DIR.dir_path).join(&fragment_file_name);

    let needs_write = if let Ok(existing) = std::fs::read_to_string(&fragment_file_path) {
        !existing.contains(&format!("<!-- litter-hash: {:07x} -->", hash))
    } else {
        true // File doesn't exist
    };

    if needs_write {
        // Create the content with the hash embedded as an HTML comment on the first line
        let md_content =
            format_fragment_md(&formatted_fragment, &fragment_name, &return_doc_name, hash);

        let _ = std::fs::create_dir_all(&FRAGMENT_DIR.dir_path);
        let _ = std::fs::write(&fragment_file_path, md_content);
    }

    item
}

// Helper function to extract an identifier from an item type.
fn get_item_ident(item: &syn::Item) -> Option<&syn::Ident> {
    match item {
        Item::Fn(i) => Some(&i.sig.ident),
        Item::Struct(i) => Some(&i.ident),
        Item::Enum(i) => Some(&i.ident),
        Item::Const(i) => Some(&i.ident),
        Item::Static(i) => Some(&i.ident),
        Item::Type(i) => Some(&i.ident),
        Item::Trait(i) => Some(&i.ident),
        Item::Mod(i) => Some(&i.ident),
        _ => None,
    }
}

// Helper to return a proper compiler error with a given message.
fn comp_error(item: &TokenStream2, err: &str) -> TokenStream {
    syn::Error::new_spanned(item, err).to_compile_error().into()
}

/// A helper to standardize formatted Markdown output.
fn format_fragment_md(code: &str, name: &str, return_doc: &str, hash: u64) -> String {
    let version_comment = format!("<!-- litter-hash: {:07x} -->\n", hash);

    // We place the link outside the code block to enable code copying,
    // and use the anchor (#name) to jump back to the exact location in the source doc.
    format!(
        "{version_comment}\
        ### Source Fragment: `{name}`\n\n\
        ```rust\n\
        {code}\n\
        ```\n\n\
        [← Back to documentation](../{return_doc}#{name})"
    )
}

#[proc_macro]
pub fn litter_anchors(item: TokenStream) -> TokenStream {
    // K: Return document, V: anchor(s)
    let mut doc_to_fragments: std::collections::HashMap<PathBuf, Vec<String>> =
        std::collections::HashMap::new();

    // Regex to extract the return-link and anchor from our own generated fragments.
    // Matches: [← Back to documentation](../README.md#anchor_name)
    let back_link_re = regex::Regex::new(
        r"\[← Back to documentation\]\((?:\./)?(?:\.\./)?(?P<doc>[^#]+)#(?P<anchor>[^)]+)\)",
    )
    .expect("back-link regex known to be valid");

    if let Ok(entries) = std::fs::read_dir(&FRAGMENT_DIR.dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            // If the extension is .md
            if path.extension().is_some_and(|e| e == "md")
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Some(caps) = back_link_re.captures(&content)
            {
                let doc_name = &caps["doc"];
                let anchor = &caps["anchor"];

                let doc_path = PROJECT_ROOT.join(doc_name);
                doc_to_fragments
                    .entry(doc_path)
                    .or_default() // existing or a new &mut Vec<String> for anchors.
                    .push(anchor.to_string());
            }
        }
    }

    // The regex crate supports named capture groups (?P<text>.+?)
    // (...) a group, ?P<text> named "text", +. matches one or more characters, ? indicates laxy matching.
    let link_re = regex::Regex::new(&format!(
        r"\[(?P<text>.+?)\]\((?:\./)?(?P<path>{}/(?P<name>[^/)]+)\.md)\)",
        FRAGMENT_DIR.escaped_dir_name
    ))
    .expect("link regex is a valid regex");

    for (doc_path, fragments) in doc_to_fragments {
        let Ok(md_content) = std::fs::read_to_string(&doc_path) else {
            continue;
        };

        let mut added_this_run = std::collections::HashSet::new();

        let maybe_updated_md = link_re.replace_all(&md_content, |caps: &regex::Captures| {
            let name = &caps["name"];
            let full_link = &caps[0]; // Full return link string
            let anchor_html = format!(r#"<a id="{}"></a>"#, name);

            // Insert anchor if the fragment exists, isn't already in the file,
            // and wasn't added yet during this macro run.
            if fragments.contains(&name.to_string())
                && !md_content.contains(&anchor_html)
                && added_this_run.insert(name.to_string())
            {
                format!("{}{}", anchor_html, full_link)
            } else {
                full_link.to_string()
            }
        });

        // If maybe_updated_md is an Owned variant, it has indeed been updated.
        if let Cow::Owned(new_content) = maybe_updated_md {
            let _ = std::fs::write(doc_path, new_content);
        }
    }

    item
}
