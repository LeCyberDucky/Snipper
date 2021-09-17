#![feature(bool_to_option)]
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{App, Arg};
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
struct Snippet {
    name: String,
    content: Option<String>,
    source_file: Option<PathBuf>,
    source: bool,
    latex: bool,
    extracted: bool,
}

impl Ord for Snippet {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Snippet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Snippet {
    fn new(
        begin: String,
        content: Option<String>,
        source_file: Option<PathBuf>,
        end: String,
        source: bool,
        latex: bool,
        extracted: bool,
    ) -> Result<Self> {
        (begin == end)
            .then_some(Self {
                name: begin.clone(),
                content,
                source_file,
                source,
                latex,
                extracted,
            })
            .context(format!(
                "Snippet with mismatched begin and end tags\n\"{}\" != \"{}\"",
                begin, end
            ))
    }
}

fn main() {
    let arguments = App::new("Snipper").about("Collects snippets of code from source files into separate files for simple inclusion in LaTeX documents.")
    .arg(Arg::with_name("Source")
        .long("Source").value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid source directory.".into()))
        .help("Root directory of source files"))
    .arg(Arg::with_name("Target")
        .long("Target")
        .value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid target directory.".into()))
        .help("Directory, where snippets will be stored"))
    .arg(Arg::with_name("LaTeX")
        .long("LaTeX").value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid LaTeX directory.".into()))
        .help("Root directory of LaTeX document")).get_matches();

    let source_directory = Path::new(arguments.value_of("Source").unwrap());
    let target_directory = Path::new(arguments.value_of("Target").unwrap());
    let latex_directory = Path::new(arguments.value_of("LaTeX").unwrap());

    let source_files =
        files_with_extension(source_directory, vec!["cpp".into(), "h".into()], false);
    let tex_files = files_with_extension(latex_directory, vec!["tex".into()], false);
    let snippet_files = files_with_extension(target_directory, vec!["cpp".into()], false);

    let snippet_pattern = regex::RegexBuilder::new(
        r"(// SNIPPET:BEGIN \{(?P<BEGIN>.*?)\}(?P<SNIPPET>.*?)// SNIPPET:END \{(?P<END>.*?)\})",
    )
    .dot_matches_new_line(true)
    .build()
    .unwrap();

    let include_pattern =
        regex::RegexBuilder::new(r"(\\lstinputlisting\{.*/(?P<SNIPPET_NAME>.*?)\.cpp.*?\})")
            .dot_matches_new_line(true)
            .build()
            .unwrap();

    let mut snippets = HashMap::new();
    for file in source_files {
        let text = fs::read_to_string(&file).context(format!("{:?}", file));
        if let Ok(text) = text {
            for captures in snippet_pattern.captures_iter(&text) {
                let snippet = Snippet::new(
                    captures["BEGIN"].into(),
                    Some(captures["SNIPPET"].into()),
                    Some(file.clone()),
                    captures["END"].into(),
                    true,
                    false,
                    false,
                );
                if let Ok(snippet) = snippet {
                    snippets.insert(snippet.name.clone(), snippet);
                } else {
                    eprintln!("{:#?}", snippet);
                }
            }
        } else {
            eprintln!("{:#?}", text);
        }
    }

    // let snippets: Vec<Result<_, anyhow::Error>> = source_files
    //     .iter()
    //     .map(|file| {
    //         let text = fs::read_to_string(file).context(format!("{:?}", file))?;
    //         Ok(snippet_pattern
    //             .captures_iter(&text)
    //             .map(|captures| {
    //                 Snippet::new(
    //                     captures["BEGIN"].into(),
    //                     Some(captures["SNIPPET"].into()),
    //                     captures["END"].into(),
    //                     true,
    //                     false,
    //                     false,
    //                 )
    //             })
    //             .collect::<Vec<_>>())
    //     })
    //     .collect();

    for file in tex_files {
        let text = fs::read_to_string(&file).context(format!("{:?}", file));
        if let Ok(text) = text {
            for captures in include_pattern.captures_iter(&text) {
                let snippet_name = &captures["SNIPPET_NAME"];
                snippets
                    .entry(snippet_name.to_owned())
                    .or_insert(
                        Snippet::new(snippet_name.to_owned(), None, None, snippet_name.to_owned(), false, true, false)
                            .expect("Failed at creating snippet from LaTeX include statement."),
                    )
                    .latex = true;
            }
        } else {
            eprintln!("{:#?}", text);
        }
    }

    for file in snippet_files {
        let snippet_name = file
            .file_name()
            .map(|name| name.to_str().map(|name| name.to_owned()))
            .flatten()
            .context("Unable to obtain snippet name from snippet file.");
        if let Ok(snippet_name) = snippet_name {
            snippets
                .entry(snippet_name.clone())
                .or_insert(
                    Snippet::new(snippet_name.to_owned(), None, None, snippet_name.to_owned(), false, false, true)
                        .expect("Failed at creating snippet from extracted snippet file."),
                )
                .extracted = true;
        } else {
            eprintln!("{:#?}", snippet_name);
        }
    }

    

    // let snippet_files: Vec<_> = snippet_files
    //     .iter()
    //     .map(|file| file.file_name().map(|name| name.to_str().to_owned()))
    //     .flatten()
    //     .collect();

    // let snippet_inclusions: Vec<Result<_, anyhow::Error>> = tex_files
    //     .iter()
    //     .map(|file| {
    //         let text = fs::read_to_string(file).context(format!("{:?}", file))?;
    //         Ok(include_pattern
    //             .captures_iter(&text)
    //             .map(|captures| captures["SNIPPET_NAME"].to_string())
    //             .collect::<Vec<_>>())
    //     })
    //     .collect();



    // 1. List found snippets and inclusions
    let mut snippets: Vec<_> = snippets.into_iter().map(|entry| entry.1).collect();
    snippets.sort();

    for (i, snippet) in snippets.iter().enumerate() {
        println!("{}.:\t{} ()\t({:?})", i + 1, snippet.name, snippet.source_file);
    }

    // println!("\nInclude statements:\n");

    // for (i, inclusion) in inclusion_collection.iter().enumerate() {
    //     println!("{}.:\t{} ({})", i + 1, inclusion.1, inclusion.0);
    // }



    // let mut snippet_collection = vec![];
    // for snippet_set in snippets.into_iter() {
    //     if let Ok(snippet_set) = snippet_set {
    //         for snippet in snippet_set.into_iter() {
    //             if let Ok(snippet) = snippet {
    //                 snippet_collection.push(snippet);
    //             } else {
    //                 eprintln!("{:#?}", snippet);
    //             }
    //         }
    //     } else {
    //         eprintln!("{:#?}", snippet_set);
    //     }
    // }

    // snippet_collection.sort();

    // let mut inclusion_collection = vec![];
    // for inclusion_set in snippet_inclusions.into_iter() {
    //     if let Ok(inclusion_set) = inclusion_set {
    //         for inclusion in inclusion_set {
    //             inclusion_collection.push(inclusion);
    //         }
    //     } else {
    //         eprintln!("{:#?}", inclusion_set);
    //     }
    // }

    // enum SnippetOccurence {
    //     Source,
    //     Include,
    //     Extraction,
    // }

    // impl std::fmt::Display for SnippetOccurence {
    //     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    //         match self {
    //             SnippetOccurence::SourceOnly => f.write_str("Source only"),
    //             SnippetOccurence::IncludeOnly => f.write_str("Include only"),
    //             SnippetOccurence::Both => f.write_str("Source and include"),
    //         }
    //     }
    // }

    // let snippet_collection: Vec<_> = snippet_collection
    //     .into_iter()
    //     .map(|snippet| {
    //         (
    //             if inclusion_collection.contains(&snippet.name) {
    //                 SnippetOccurence::Both
    //             } else {
    //                 SnippetOccurence::SourceOnly
    //             },
    //             snippet,
    //         )
    //     })
    //     .collect();
    // let inclusion_collection: Vec<_> = inclusion_collection
    //     .into_iter()
    //     .map(|inclusion| {
    //         (
    //             if snippet_collection
    //                 .iter()
    //                 .map(|snippet| &snippet.1.name)
    //                 .collect::<Vec<_>>()
    //                 .contains(&&inclusion)
    //             {
    //                 SnippetOccurence::Both
    //             } else {
    //                 SnippetOccurence::IncludeOnly
    //             },
    //             inclusion,
    //         )
    //     })
    //     .collect();

    // for (i, snippet) in snippet_collection.iter().enumerate() {
    //     println!("{}.:\t{} ({})", i + 1, snippet.1.name, snippet.0);
    // }

    // println!("\nInclude statements:\n");

    // for (i, inclusion) in inclusion_collection.iter().enumerate() {
    //     println!("{}.:\t{} ({})", i + 1, inclusion.1, inclusion.0);
    // }
}

fn files_with_extension(
    root_directory: &Path,
    mut extensions: Vec<String>,
    case_sensitive: bool,
) -> Vec<PathBuf> {
    if !case_sensitive {
        for x in extensions.iter_mut() {
            x.make_ascii_lowercase()
        }
    }

    WalkDir::new(root_directory)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_type().is_dir()
                || (entry.file_type().is_file()
                    && extensions
                        .iter()
                        .map(|extension| Some(extension.to_string()))
                        .any(|x| {
                            x == entry
                                .path()
                                .extension()
                                .map(|extension| {
                                    if !case_sensitive {
                                        extension.to_owned().make_ascii_lowercase();
                                    }
                                    extension.to_str().map(|x| x.to_string())
                                })
                                .flatten()
                        }))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().is_file().then_some(entry.into_path())
        })
        .collect()
}
