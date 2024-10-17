# table-rustractor

`table-rustractor` is a command-line tool written in Rust for extracting tables
from HTML files or URLs and saving them as CSV files. This tool is designed to
help you efficiently parse and process HTML tables into a more manageable CSV
format.

`table-rustractor` fetches HTML content from a specified file path or URL,
extracts all tables found within the HTML, and saves each table as a separate
CSV file in the specified output directory. The tool supports the following
features:

- HTML Source Flexibility: Fetch HTML content from both local files and remote URLs.
- Table Extraction: Parse and extract tables, including handling colspan and rowspan attributes correctly.
- Debug Mode: Enable debug mode for detailed logs and insights during the extraction process.

## Installation

To install `table-rustractor`, you'll need to have Rust and Cargo installed.
You can install Rust and Cargo using [rustup](https://rustup.rs/).

Once Rust is installed, you can install `table-rustractor` by running:

```shell
cargo install table-rustractor
```

## Usage

The `table-rustractor` CLI allows you to extract tables from HTML files or URLs and save them as CSV files.

### Arguments

- `-i, --input <INPUT>`: Input HTML file path or URL (required).
- `-o, --output-dir <OUTPUT_DIR>`: Output directory for CSV files (default: current directory).
- `-d, --debug`: Enable debug mode for detailed output.

### Examples

Extract tables from a local HTML file and save them to the current directory:

```shell
table-rustractor -i path/to/your/file.html
```

Extract tables from a URL and save them to a specified directory:

```shell
table-rustractor -i https://example.com -o output/directory
```

Enable debug mode to get detailed output while processing:

```shell
table-rustractor -i path/to/your/file.html -d
```

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests on
the GitHub repository.

## License

This project is licensed under the MIT License.
