# Snipper
## Tool for extracting snippets of code from source files into their own files for easy inclusion in LaTeX documents.
```
Manages snippets of source code for simple inclusion in LaTeX documents, by collecting snippets of code from source
files into separate files.
Snippets in source files are found according to the pattern

// SNIPPET:BEGIN {Worksheet 1 - A}${Optional comment about snippet}
SNIPPET CONTENT
// SNIPPET:END {Worksheet 1 - A}

Snippet inclusions in LaTeX documents are found according to the pattern

\lstinputlisting[label = {Snippet/1/A}, caption = {}, captionpos = b]{"Content/Snippets/Worksheet 1 - A.cpp"}

where the arguments are optional, and only the stem of the file path is important.

Snippets can be marked as inactive by placing underscores in front of the snippet tags: _SNIPPET:BEGIN ... _SNIPPET:END.
The already extracted snippet files of inactive snippets will not be overwritten on extraction.

USAGE:
    snipper.exe [FLAGS] --LaTeX <DIRECTORY> --Source <DIRECTORY> --Target <DIRECTORY>

FLAGS:
        --Extract    Extract found snippets into separate snippet files for inclusion in LaTeX document
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --LaTeX <DIRECTORY>     Root directory of LaTeX document
        --Source <DIRECTORY>    Root directory of source files
        --Target <DIRECTORY>    Directory, where snippets will be stored
```

## Getting started
The tool is built in [Rust](https://www.rust-lang.org/). Having installed Rust, download the repository and then build/run the tool by running the following command in the directory:

`cargo +nightly run -- -h`
