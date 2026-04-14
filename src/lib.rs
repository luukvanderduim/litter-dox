use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
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
static DOX_INFO: LazyLock<DoxInfo> = LazyLock::new(|| {
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

    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .expect("path always resolves to path");

    DoxInfo {
        path,
        escaped_folder: regex::escape(&folder_name),
    }
});

/// Path to the directory where code fragment files are stored and the escaped folder name for use in regex patterns.
/// This is set to the `LITTER_DOX_PATH` environment variable, or `"litdox"` if that is not set and if those two fail, the current directory `"."` is used.
struct DoxInfo {
    path: PathBuf,
    escaped_folder: String,
}

// A type to encapsulate the parsed relevant input.
enum ParsedInput {
    /// Represents a collection of items. While custom inner attributes (#![litter])
    /// are unstable for proc-macros on stable Rust, syn::File is still used
    /// to represent a sequence of multiple items or a virtual file buffer.
    File(Box<syn::File>),

    /// A single named Rust item (struct, fn, enum, etc.)
    Item(Box<syn::Item>),

    /// A code fragment that doesn't form a complete item (e.g., a block or expression).
    Fragment(Box<TokenStream2>),
}

impl ParsedInput {
    /// Format to Rust code string.
    fn to_formatted_string(&self) -> String {
        match self {
            // File obtained with inner attributes #![litter]
            // Or in case of function-like macros, if a macro encapsulates multiple items
            ParsedInput::File(f) => prettyplease::unparse(f),

            // Case 2: Named item (fn, struct, etc.)
            ParsedInput::Item(i) => {
                // We wrap the item in a temporary File so prettyplease can handle it
                let file = syn::File {
                    shebang: None,
                    attrs: vec![],
                    items: vec![(**i).clone()],
                };
                prettyplease::unparse(&file)
            }

            // Case 3: Raw fragment (blocks, closures, etc.)
            ParsedInput::Fragment(tokens) => {
                // For raw fragments, we use the string representation.
                // Since the TokenStream is "sealed" (braces included), it remains valid.
                tokens.to_string()
            }
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
    // Rustc provides `TokenStream` which becomes available when the library is set to be of proc-macro type.
    // That type, however is an opaque type, `proc_macro2::TokenStream` is a more ergonomic and malleable type.
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
        // Name provided, and it's a full file
        (Some(n), Ok(f), _) => (n, ParsedInput::File(Box::new(f))),

        // Name provided, and it's an item
        (Some(n), _, Ok(i)) => (n, ParsedInput::Item(Box::new(i))),

        // No name, but we may have a named Item
        (None, _, Ok(i)) => {
            if let Some(ident) = get_item_ident(&i) {
                (ident.to_string(), ParsedInput::Item(Box::new(i)))
            } else {
                return comp_error(
                    &item2,
                    "A name is required for this type of item (e.g. impl blocks).",
                );
            }
        }

        // Name provided, but parsing as File/Item failed (it's a fragment)
        (Some(n), _, _) => (n, ParsedInput::Fragment(Box::new(item2.clone()))),

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

    let pretty_code = parsed_input.to_formatted_string();

    // The fragment file name will comprise of name and short hash, so let's compute a hash.
    let mut hasher = DefaultHasher::new();
    pretty_code.hash(&mut hasher);
    // Mask against 28 LSBs, 4 bits equals a hex digit, thus 7 hex digits
    let hash = hasher.finish() & 0x0FFF_FFFF;

    let file_name = format!("{}_{:07x}.md", fragment_name, hash);
    let symlink_name = format!("{}.md", fragment_name);

    let path = std::path::PathBuf::from(&DOX_INFO.path).join(&file_name);
    let symlink_path = std::path::PathBuf::from(&DOX_INFO.path).join(&symlink_name);

    // Is the symlink already up-to-date?
    let symlink_up_to_date = std::fs::read_link(&symlink_path)
        .map(|target| target == std::path::Path::new(&file_name))
        .unwrap_or(false);

    if !symlink_up_to_date {
        // Create fragment md
        if !path.exists() {
            let md = format_fragment_md(&pretty_code, &fragment_name, &return_doc_name);
            std::fs::create_dir_all(&DOX_INFO.path).ok();
            std::fs::write(&path, md).ok();
        }

        // Delete old symlink target
        if let Ok(old_target_name) = std::fs::read_link(&symlink_path) {
            let old_target_path = DOX_INFO.path.join(old_target_name);

            // `symlink_up_to_date` is false if:
            // - the old target exists and is not the new target
            // - if the symlink was manually changed
            // - on first run, symlink does not exist
            if old_target_path.exists() {
                let _ = std::fs::remove_file(old_target_path);
            }
        }

        // Refresh the symlink:
        let _ = std::fs::remove_file(&symlink_path);

        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&file_name, &symlink_path);

        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_file(&file_name, &symlink_path);
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
fn format_fragment_md(code: &str, name: &str, return_doc: &str) -> String {
    // We place the link outside the code block to enable code copying,
    // and use the anchor (#name) to jump back to the exact location in the source doc.
    format!(
        "### Source Fragment: `{name}`\n\n\
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

    if let Ok(entries) = std::fs::read_dir(&DOX_INFO.path) {
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
        DOX_INFO.escaped_folder
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
